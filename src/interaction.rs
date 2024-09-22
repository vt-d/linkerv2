#![allow(clippy::option_as_ref_deref)]

pub mod music;
pub mod ping;

use std::{mem, sync::Arc};

use anyhow::{bail, Result};
use music::MusicCommand;
use ping::PingCommand;
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::Event;
use twilight_model::{
    application::interaction::{application_command::CommandData, Interaction, InteractionData},
    channel::message::Embed,
    http::interaction::{InteractionResponse, InteractionResponseType},
};
use twilight_util::builder::{embed::EmbedBuilder, InteractionResponseDataBuilder};

use crate::State;

pub(super) struct CommandContext {
    interaction: Interaction,
    data: CommandData,
    state: State,
    cache: Arc<InMemoryCache>,
    handled: bool,
}

impl CommandContext {
    pub async fn defer(&mut self) -> Result<()> {
        let client = self.state.http.interaction(self.interaction.application_id);
        let response = InteractionResponse {
            kind: InteractionResponseType::DeferredChannelMessageWithSource,
            data: None,
        };

        client
            .create_response(self.interaction.id, &self.interaction.token, &response)
            .await?;

        self.handled = true;
        Ok(())
    }

    pub async fn reply(&self, response: InteractionResponse) -> Result<()> {
        let client = self.state.http.interaction(self.interaction.application_id);
        if !self.handled {
            client
                .create_response(self.interaction.id, &self.interaction.token, &response)
                .await?;

            return Ok(());
        }

        if response.data.is_none() {
            bail!("Response data is None")
        };

        let data = response.data.unwrap_or_default();
        let embed_vec = data.embeds.unwrap_or_default();

        let embeds: Box<[Embed]> = embed_vec.into();
        let content = data.content.as_ref().map(|cont| cont.as_str());

        client
            .update_response(&self.interaction.token)
            .embeds(Some(&*embeds))
            .content(content)
            .await?;

        Ok(())
    }
}

trait Handleable: Sized {
    async fn handle(ctx: CommandContext) -> Result<()>;
}

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
    state: State,
    cache: Arc<InMemoryCache>,
) -> anyhow::Result<()> {
    let command_ctx = CommandContext {
        interaction,
        data: data.clone(),
        state,
        cache,
        handled: false,
    };

    match &*data.name {
        "ping" => PingCommand::handle(command_ctx).await,
        "music" => MusicCommand::handle(command_ctx).await,
        name => bail!("unknown command: {}", name),
    }
}
