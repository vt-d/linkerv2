use anyhow::bail;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_util::builder::embed::EmbedBuilder;

use crate::interaction::CommandContext;

#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "pause", desc = "Pause the current song playing in VC")]
pub struct MusicPause;

impl MusicPause {
    pub async fn run(&self, ctx: CommandContext) -> anyhow::Result<()> {
        let client = ctx.state.http.interaction(ctx.interaction.application_id);
        let guild_id = match ctx.interaction.guild_id {
            Some(guild_id) => guild_id,
            None => {
                bail!("You must be in a guild to execute this command!");
            }
        };

        let voice_state = ctx
            .cache
            .voice_state(ctx.interaction.author_id().unwrap(), guild_id);

        let channel_id = match voice_state {
            Some(voice_state) => voice_state.channel_id(),
            None => {
                bail!("You must be in a voice channel to execute this command!");
            }
        };

        let call_lock;
        let get_call = ctx.state.songbird.get(guild_id);
        let call = {
            call_lock = match get_call {
                Some(get_call) => get_call,
                None => ctx.state.songbird.join(guild_id, channel_id).await.unwrap(),
            };

            call_lock.lock().await
        };

        call.queue().pause()?;

        let embed = EmbedBuilder::new().title(":white_check_mark: `/pause` - Success");

        client
            .update_response(&ctx.interaction.token)
            .embeds(Some(&[embed.build()]))
            .await?;

        Ok(())
    }
}
