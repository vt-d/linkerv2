use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    application::interaction::{application_command::CommandData, Interaction},
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
};

use crate::State;

#[derive(CommandModel, CreateCommand)]
#[command(name = "ping", desc = "Pong")]
pub struct PingCommand;

impl PingCommand {
    pub async fn handle(
        interaction: Interaction,
        _: CommandData,
        state: State,
    ) -> anyhow::Result<()> {
        let client = state.http.interaction(interaction.application_id);
        let response = InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData {
                content: Some("Pong!".to_string()),
                ..Default::default()
            }),
        };

        client
            .create_response(interaction.id, &interaction.token, &response)
            .await?;

        Ok(())
    }
}
