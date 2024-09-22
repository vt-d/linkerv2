use anyhow::Context;
use twilight_interactions::command::{CommandModel, CreateCommand};

use super::{CommandContext, Handleable};

mod pause;
mod play;
mod resume;

// TODO: get_call function

#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "music", desc = "Play music in VC!")]
pub enum MusicCommand {
    #[command(name = "play")]
    Play(play::MusicPlay),
    #[command(name = "pause")]
    Pause(pause::MusicPause),
    #[command(name = "resume")]
    Resume(resume::MusicResume),
}

impl Handleable for MusicCommand {
    async fn handle(ctx: CommandContext) -> anyhow::Result<()> {
        let command = MusicCommand::from_interaction(ctx.data.clone().into())
            .context("failed to parse command data")?;

        match command {
            MusicCommand::Play(command) => command.run(ctx).await,
            MusicCommand::Pause(command) => command.run(ctx).await,
            MusicCommand::Resume(command) => command.run(ctx).await,
        }
    }
}
