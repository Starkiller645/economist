use serenity::model::application::interaction::application_command::CommandDataOption;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;

pub fn get_options(data: &ApplicationCommandInteraction) -> Result<Vec<CommandDataOption>, String> {
    let command = match data.data.options.get(0) {
        Some(a) => a,
        None => return Err("Couldn't get subcommand data: indirection layer 1".into()),
    };

    let action = match command.options.get(0) {
        Some(a) => {
            if a.options.len() != 0 {
                a
            } else {
                return Ok(command.options.clone());
            }
        }
        None => return Ok(command.options.clone()),
    };

    Ok(action.options.clone())
}
