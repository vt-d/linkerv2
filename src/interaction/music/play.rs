use std::sync::Arc;

use anyhow::bail;
use songbird::input::{Compose, YoutubeDl};
use twilight_cache_inmemory::InMemoryCache;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    application::interaction::{application_command::CommandData, Interaction},
    http::interaction::{InteractionResponse, InteractionResponseType},
};
use twilight_util::builder::embed::{EmbedBuilder, ImageSource};

use crate::State;

#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "play", desc = "Play music in VC!")]
pub struct MusicPlay {
    #[command(desc = "Search term to find songs/videos.")]
    pub query: String, // twilight-interactions RAHHH
}

impl MusicPlay {
    pub async fn run(
        &self,
        interaction: Interaction,
        _: CommandData,
        state: State,
        cache: Arc<InMemoryCache>,
    ) -> anyhow::Result<()> {
        tracing::info!("Plinged");
        let client = state.http.interaction(interaction.application_id);
        let response = InteractionResponse {
            kind: InteractionResponseType::DeferredChannelMessageWithSource,
            data: None,
        };

        client
            .create_response(interaction.id, &interaction.token, &response)
            .await?;

        tracing::info!("Plinged");

        let query = self.query.clone();

        let guild_id = match interaction.guild_id {
            Some(guild_id) => guild_id,
            None => {
                bail!("You must be in a guild to execute this command!");
            }
        };

        let voice_state = cache.voice_state(interaction.author_id().unwrap(), guild_id);
        let channel_id = match voice_state {
            Some(voice_state) => voice_state.channel_id(),
            None => {
                bail!("You must be in a voice channel to execute this command!");
            }
        };

        let call_lock;

        let mut call = {
            let get_call = state.songbird.get(guild_id);
            call_lock = match get_call {
                Some(get_call) => get_call,
                None => state.songbird.join(guild_id, channel_id).await.unwrap(),
            };

            call_lock.lock().await
        };

        let mut src: YoutubeDl;
        if query.starts_with("http") {
            src = YoutubeDl::new(reqwest::Client::new(), query);
        } else {
            src = YoutubeDl::new_search(reqwest::Client::new(), query);
        }
        let metadata = src.aux_metadata().await.unwrap();

        let _song = call.enqueue_input(src.into()).await;

        let queue_len = call.queue().len();
        tracing::info!("{}", queue_len);

        // todo add proper handling for
        client
            .update_response(&interaction.token)
            .embeds(Some(&[EmbedBuilder::new()
                .title(":white_check_mark: `/play` - Success")
                .description(format!(
                    ":arrow_forward: [`{} - {}`]({})",
                    metadata.artist.unwrap_or(String::from("")),
                    metadata.title.unwrap_or(String::from("")),
                    metadata.source_url.unwrap_or(String::from(""))
                ))
                .image(ImageSource::url(
                    metadata.thumbnail.unwrap_or(String::from("")),
                )?)
                .build()]))
            .await?;

        Ok(())
    }
}
