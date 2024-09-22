use anyhow::bail;
use songbird::input::{Compose, YoutubeDl};
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::{
    embed::{EmbedBuilder, ImageSource},
    InteractionResponseDataBuilder,
};

use super::CommandContext;

#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "play", desc = "Play music in VC!")]
pub struct MusicPlay {
    #[command(desc = "Search term to find songs/videos.")]
    pub query: String, // twilight-interactions RAHHH
}

impl MusicPlay {
    pub async fn run(&self, mut ctx: CommandContext) -> anyhow::Result<()> {
        let query = self.query.clone();
        let guild_id = match ctx.interaction.guild_id {
            Some(guild_id) => guild_id,
            None => {
                bail!("You must be in a guild to execute this command!");
            }
        };

        let channel_id = match ctx
            .cache
            .clone() // Arc<InMemoryCache> not expensive
            .voice_state(ctx.interaction.author_id().unwrap(), guild_id)
        {
            Some(voice_state) => voice_state.channel_id(),
            None => {
                bail!("You must be in a voice channel to execute this command!");
            }
        };

        ctx.defer().await?;

        let call_lock;
        let get_call = ctx.state.songbird.get(guild_id);

        let mut call = {
            call_lock = match get_call {
                Some(get_call) => get_call,
                None => ctx.state.songbird.join(guild_id, channel_id).await.unwrap(),
            };

            call_lock.lock().await
        };

        let mut src: YoutubeDl;
        if query.starts_with("http") {
            src = YoutubeDl::new(ctx.state.reqwest.clone(), query); // this is what serenity does;
                                                                    // expensive ?
        } else {
            src = YoutubeDl::new_search(ctx.state.reqwest.clone(), query);
        }

        let metadata = src.aux_metadata().await.unwrap();
        let _song = call.enqueue_input(src.into()).await;

        let mut embed = EmbedBuilder::new()
            .title(":white_check_mark: `/play` - Success")
            .description(format!(
                "Artist: **{}**\nTitle: [`{}`]({})",
                metadata.artist.as_deref().unwrap_or(""),
                metadata.title.as_deref().unwrap_or(""),
                metadata.source_url.as_deref().unwrap_or(""),
            ));

        if metadata.thumbnail.is_some() {
            embed = embed.image(ImageSource::url(metadata.thumbnail.unwrap()).unwrap());
        }

        let response = InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(
                InteractionResponseDataBuilder::new()
                    .embeds([embed.build()])
                    .build(),
            ),
        };

        ctx.reply(response).await?;

        Ok(())
    }
}
