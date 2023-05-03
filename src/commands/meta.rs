use crate::consts::*;
use crate::CommandResponseObject;
use rustc_version::version;
use serenity::builder::{CreateApplicationCommand, CreateComponents};
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("economist")
        .description("Economist: Get version debug information")
        .create_option(|option| {
            option
                .kind(CommandOptionType::SubCommand)
                .name("version")
                .description("Get version and build debug information")
        })
}

pub fn run(data: &ApplicationCommandInteraction) -> CommandResponseObject {
    let subcommand_data = match data.data
            .options.get(0) {
                Some(data) => data,
                None => return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: Economist Bot encountered an error processing a command. A debug dump of the command interaction is below:\n{data:?}"), false)
            };

    match subcommand_data.name.as_str() {
        "version" => {
            let bot_version = match GIT_VERSION {
                Some(version) => {
                    if version.contains("v") && version.contains(".") {
                        version
                    } else {
                        PKG_VERSION
                    }
                }
                None => PKG_VERSION,
            };

            let rustc_info = version().unwrap();
            let os = std::env::consts::OS;

            CommandResponseObject::text(format!(
                "**Economist Bot**, written by @Starkiller645
> Version: **{bot_version}**
> Build time: `{BUILT_TIME_UTC}`
> Target: `{TARGET}`
> Host: `{HOST}`
> rustc: `{rustc_info}`, on `{os}`, `{CFG_ENV}` toolchain",
            ))
        }
        _ => CommandResponseObject::text(""),
    }
}
