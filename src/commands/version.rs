use rustc_version::{version, version_meta, Channel, Version};
use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
};

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("version")
        .description("Economist: Get version debug information")
}

pub fn run(data: &ApplicationCommandInteraction) -> String {
    let bot_version = git_version::git_version!();

    let rustc_info = version().unwrap();

    format!(
        "**Economist Bot**, written by @Starkiller645
        Version `{}`
        rustc: `{}`, on `{}`",
        bot_version,
        rustc_info,
        std::env::consts::OS
    )
}
