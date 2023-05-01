use crate::CommandResponseObject;
use rustc_version::version;
use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("version")
        .description("Economist: Get version debug information")
}

pub fn run(_data: &ApplicationCommandInteraction) -> CommandResponseObject {
    let bot_version = git_version::git_version!();

    let rustc_info = version().unwrap();

    CommandResponseObject::text(format!(
        "**Economist Bot**, written by @Starkiller645
Version `{}`
rustc: `{}`, on `{}`",
        bot_version,
        rustc_info,
        std::env::consts::OS
    ))
}
