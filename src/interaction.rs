pub mod music;
pub mod ping;

use std::mem;

use anyhow::bail;
use ping::PingCommand;
use twilight_gateway::Event;
use twilight_model::application::interaction::{
    application_command::CommandData, Interaction, InteractionData,
};

use crate::State;

pub async fn process_interactions(event: Event, client: State) {
    let mut interaction = match event {
        Event::InteractionCreate(interaction) => interaction.0,
        _ => return,
    };

    let data = match mem::take(&mut interaction.data) {
        Some(InteractionData::ApplicationCommand(data)) => *data,
        _ => {
            tracing::warn!("ignoring non-command interaction");
            return;
        }
    };

    if let Err(error) = handle_command(interaction, data, client).await {
        tracing::error!(?error, "error while handling command");
    }
}

async fn handle_command(
    interaction: Interaction,
    data: CommandData,
    client: State,
) -> anyhow::Result<()> {
    match &*data.name {
        "ping" => PingCommand::handle(interaction, data, client).await,
        name => bail!("unknown command: {}", name),
    }
}
