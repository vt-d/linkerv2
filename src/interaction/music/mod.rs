use std::sync::Arc;

use anyhow::Context;
use twilight_cache_inmemory::InMemoryCache;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::application::interaction::{application_command::CommandData, Interaction};

use crate::State;

pub mod play;

#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "music", desc = "Play music in VC!")]
pub enum MusicCommand {
    #[command(name = "play")]
    Play(play::MusicPlay),
}

impl MusicCommand {
    pub async fn handle(
        interaction: Interaction,
        data: CommandData,
        state: State,
        cache: Arc<InMemoryCache>,
    ) -> anyhow::Result<()> {
        let command = MusicCommand::from_interaction(data.clone().into())
            .context("failed to parse command data")?;

        match command {
            MusicCommand::Play(command) => command.run(interaction, data, state, cache).await,
        }
    }
}
