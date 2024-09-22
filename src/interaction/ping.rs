use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};

use super::{CommandContext, Handleable};

#[derive(CommandModel, CreateCommand)]
#[command(name = "ping", desc = "Pong")]
pub struct PingCommand;

impl Handleable for PingCommand {
    async fn handle(ctx: CommandContext) -> anyhow::Result<()> {
        let client = ctx.state.http.interaction(ctx.interaction.application_id);
        let response = InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData {
                content: Some("Pong!".to_string()),
                ..Default::default()
            }),
        };

        client
            .create_response(ctx.interaction.id, &ctx.interaction.token, &response)
            .await?;

        Ok(())
    }
}
