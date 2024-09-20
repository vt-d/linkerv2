mod interaction;

use std::{
    collections::HashMap,
    env,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use dotenvy::dotenv;
use interaction::process_interactions;
use songbird::{shards::TwilightMap, tracks::TrackHandle, Songbird};
use tokio::sync::RwLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use twilight_gateway::{
    error::ReceiveMessageErrorType, CloseFrame, ConfigBuilder, Event, EventTypeFlags, Intents,
    Shard, StreamExt as _,
};
use twilight_http::Client;
use twilight_interactions::command::CreateCommand;
use twilight_model::{
    gateway::{
        payload::outgoing::update_presence::UpdatePresencePayload,
        presence::{ActivityType, MinimalActivity, Status},
    },
    id::{marker::GuildMarker, Id},
};
use twilight_standby::Standby;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

type State = Arc<StateRef>;

#[derive(Debug)]
struct StateRef {
    http: Arc<Client>,
    trackdata: RwLock<HashMap<Id<GuildMarker>, TrackHandle>>,
    songbird: Songbird,
    standby: Standby,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let token = env::var("TOKEN")?;

    let fmt_tracing_layer = tracing_subscriber::fmt::layer().without_time().pretty();
    tracing_subscriber::registry()
        .with(fmt_tracing_layer)
        .with(tracing_journald::layer()?)
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=trace", env!("CARGO_CRATE_NAME")).into()),
        )
        .try_init()?;

    let client = Arc::new(Client::new(token.clone()));
    let config = ConfigBuilder::new(token.clone(), Intents::GUILD_VOICE_STATES)
        .presence(presence())
        .build();

    let commands = [interaction::ping::PingCommand::create_command().into()];
    let application = client.current_user_application().await?.model().await?;
    let interaction_client = client.interaction(application.id);

    let (shards, state) = {
        let user_id = client.current_user().await?.model().await?.id;

        let shards: Vec<Shard> =
            twilight_gateway::create_recommended(&client, config, |_, builder| builder.build())
                .await?
                .collect();

        let senders = TwilightMap::new(
            shards
                .iter()
                .map(|s| (s.id().number(), s.sender()))
                .collect(),
        );

        let songbird = Songbird::twilight(Arc::new(senders), user_id);

        (
            shards,
            Arc::new(StateRef {
                http: client.clone(),
                trackdata: Default::default(),
                songbird,
                standby: Standby::new(),
            }),
        )
    };

    tracing::info!("logged in as {}", application.name);

    if let Err(error) = interaction_client.set_global_commands(&commands).await {
        tracing::error!(?error, "failed to register commands");
    }

    let shard_len = shards.len();
    let mut senders = Vec::with_capacity(shard_len);
    let mut tasks = Vec::with_capacity(shard_len);

    for shard in shards {
        senders.push(shard.sender());
        tasks.push(tokio::spawn(runner(shard, state.clone())));
    }

    tokio::signal::ctrl_c().await?;
    SHUTDOWN.store(true, Ordering::Relaxed);
    for sender in senders {
        _ = sender.close(CloseFrame::NORMAL);
    }

    for jh in tasks {
        _ = jh.await;
    }

    Ok(())
}

async fn runner(mut shard: Shard, state: State) {
    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let event = match item {
            Ok(Event::GatewayClose(_)) if SHUTDOWN.load(Ordering::Relaxed) => break,
            Ok(event) => event,
            Err(error)
                if SHUTDOWN.load(Ordering::Relaxed)
                    && matches!(error.kind(), ReceiveMessageErrorType::WebSocket) =>
            {
                break
            }
            Err(error) => {
                tracing::warn!(?error, "error while receiving event");
                continue;
            }
        };

        state.standby.process(&event);
        state.songbird.process(&event).await;

        tracing::info!(kind = ?event.kind(), shard = ?shard.id().number(), "received event");
        tokio::spawn(process_interactions(event, state.clone()));
    }
}

fn presence() -> UpdatePresencePayload {
    let activity = MinimalActivity {
        kind: ActivityType::Watching,
        name: String::from("linker's basement"),
        url: None,
    };

    UpdatePresencePayload {
        activities: vec![activity.into()],
        afk: false,
        since: None,
        status: Status::Online,
    }
}
