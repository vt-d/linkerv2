mod interaction;

use std::{
    env,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use dotenvy::dotenv;
use interaction::{music::MusicCommand, process_interactions};
use songbird::{shards::TwilightMap, Songbird};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use twilight_cache_inmemory::{DefaultInMemoryCache, InMemoryCache, ResourceType};
use twilight_gateway::{
    error::ReceiveMessageErrorType, CloseFrame, ConfigBuilder, Event, EventTypeFlags, Intents,
    Shard, StreamExt as _,
};
use twilight_http::Client;
use twilight_interactions::command::CreateCommand;
use twilight_model::gateway::{
    payload::outgoing::update_presence::UpdatePresencePayload,
    presence::{ActivityType, MinimalActivity, Status},
};
use twilight_standby::Standby;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

type State = Arc<StateRef>;

struct StateRef {
    http: Arc<Client>,
    songbird: Songbird,
    standby: Standby,
    reqwest: reqwest::Client,
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
    let config = ConfigBuilder::new(token.clone(), Intents::GUILD_VOICE_STATES | Intents::GUILDS)
        .presence(presence())
        .build();

    let commands = [
        interaction::ping::PingCommand::create_command().into(),
        MusicCommand::create_command().into(),
    ];
    let application = client.current_user_application().await?.model().await?;
    let interaction_client = client.interaction(application.id);

    let (shards, state) = {
        let user_id = client.current_user().await?.model().await?.id;

        let shards: Vec<(Arc<InMemoryCache>, Shard)> =
            twilight_gateway::create_recommended(&client, config, |_, builder| builder.build())
                .await?
                .map(|shard| {
                    let cache = Arc::new(
                        DefaultInMemoryCache::builder()
                            .resource_types(ResourceType::all())
                            .message_cache_size(8)
                            .build(),
                    );

                    (cache, shard)
                })
                .collect();

        let senders = TwilightMap::new(
            shards
                .iter()
                .map(|s| (s.1.id().number(), s.1.sender()))
                .collect(),
        );

        let songbird = Songbird::twilight(Arc::new(senders), user_id);
        // let id = shards.first().unwrap().0.current_user().unwrap().id; // this should
        // process.exit()

        (
            shards,
            Arc::new(StateRef {
                http: client.clone(),
                songbird,
                standby: Standby::new(), // not required for now but good to have
                reqwest: reqwest::Client::new(),
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
        senders.push(shard.1.sender());
        tasks.push(tokio::spawn(runner(shard.1, state.clone(), shard.0)));
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

async fn runner(mut shard: Shard, state: State, cache: Arc<InMemoryCache>) {
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

        cache.update(&event);

        tracing::info!(kind = ?event.kind(), shard = ?shard.id().number(), "received event");
        tokio::spawn(process_interactions(event, state.clone(), cache.clone()));
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
