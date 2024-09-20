pub mod music;
pub mod ping;

use std::{mem, sync::Arc};

use anyhow::bail;
use music::MusicCommand;
use ping::PingCommand;
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::Event;
use twilight_model::{
    application::interaction::{application_command::CommandData, Interaction, InteractionData},
    http::interaction::{InteractionResponse, InteractionResponseType},
};
use twilight_util::builder::{embed::EmbedBuilder, InteractionResponseDataBuilder};

use crate::State;

pub async fn process_interactions(
    event: Event,
    client: State,
    cache: Arc<InMemoryCache>,
) -> anyhow::Result<()> {
    let mut interaction = match event {
        Event::InteractionCreate(interaction) => interaction.0,
        _ => return Ok(()),
    };

    let data = match mem::take(&mut interaction.data) {
        Some(InteractionData::ApplicationCommand(data)) => *data,
        _ => {
            tracing::warn!("ignoring non-command interaction");
            return Ok(());
        }
    };

    if let Err(error) = handle_command(interaction.clone(), data, client.clone(), cache).await {
        let client = client.http.interaction(interaction.application_id);
        let response = InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(
                InteractionResponseDataBuilder::new()
                    .embeds([EmbedBuilder::new()
                        .title(":warning: Error")
                        .description(error.to_string())
                        .build()])
                    .build(),
            ),
        };

        client
            .create_response(interaction.id, &interaction.token, &response)
            .await?;
        tracing::error!(?error, "error while handling command");
    }

    Ok(())
}

async fn handle_command(
    interaction: Interaction,
    data: CommandData,
    client: State,
    cache: Arc<InMemoryCache>,
) -> anyhow::Result<()> {
    match &*data.name {
        "ping" => PingCommand::handle(interaction, data, client).await,
        "music" => MusicCommand::handle(interaction, data, client, cache).await,
        name => bail!("unknown command: {}", name),
    }
}
