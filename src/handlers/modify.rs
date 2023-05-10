use crate::commands::manage::*;
use crate::commands::query::*;
use crate::types::*;
use crate::utils;
use crate::CommandResponseObject;
use async_trait::async_trait;
use serenity::model::prelude::command::CommandOptionType;
use serenity::builder::{CreateComponents, CreateApplicationCommandOption};
use serenity::model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
};

pub struct ModifyHandler {}
struct ModifyOptions {
    code: Option<String>,
    name: Option<String>,
    state: Option<String>,
    old_code: Option<String>,
    new_code: Option<String>
}

#[async_trait]
impl ApplicationCommandHandler for ModifyHandler {
    async fn handle_application_command(&mut self, data: &ApplicationCommandInteraction, _query_agent: &DBQueryAgent, manager: &DBManager) -> Result<CommandResponseObject, String> {

        let option_data = match utils::get_options(&data) {
            Ok(o) => o,
            Err(e) => return Err(format!("Error while getting options from command data: {e:?}"))
        };

        let options: ModifyOptions = match self.parse_options(&option_data) {
            Ok(o) => o,
            Err(e) => return Err(format!("Error while parsing options: {e:?}"))
        };

        let cmd = match data.data.options.get(0) {
            Some(c) => c,
            None => return Err("Error while parsing options: Couldn't get subcommand data".into())
        };

        let action = match cmd.options.get(0) {
            Some(a) => a.name.clone(),
            None => return Err("Error while parsing options: Couldn't get which sub-subcommand to run".into())
        };

        let mut final_data = Err(sqlx::Error::Protocol("Error in command arguments".into()));

        match action.as_str() {
            "code" => {
                if let Some(old_code) = options.old_code {
                    if let Some(new_code) = options.new_code {
                        final_data = manager.modify_currency_meta(old_code, ModifyMetaType::Code, new_code).await;
                    }
                }
            },
            "state" => {
                if let Some(code) = options.code {
                    if let Some(state) = options.state {
                        final_data = manager.modify_currency_meta(code, ModifyMetaType::State, state).await;
                    }
                }
            },
            "name" => {
                if let Some(code) = options.code {
                    if let Some(name) = options.name {
                        final_data = manager.modify_currency_meta(code, ModifyMetaType::Name, name).await
                    }
                }
            },
            _ => {}
        }

        let currency_data = match final_data {
            Ok(data) => data,
            Err(e) => return Err(format!("Error while updating currency data: {e:?}"))
        };

        let currency_display = match action.as_str() {
            "code" | "state" => format!("**{}**", currency_data.currency_name),
            _ => format!("`{}`", currency_data.currency_code)
        };

        let modification = match action.as_str() {
            "code" => format!("Currency Code -> `{}`", currency_data.currency_code),
            "state" => format!("Nation/State -> *{}*", currency_data.state),
            "name" => format!("Currency Name -> **{}**", currency_data.currency_name),
            _ => "".into()
        };

        let feedback = format!("Successfully modified currency **{}** `{}`", currency_data.currency_name, currency_data.currency_code);

        Ok(CommandResponseObject::interactive_with_feedback(
            CreateComponents::default(),
            feedback,
            format!("{0} modified currency {1}:\n> {2}",
                data.user,
                currency_display,
                modification
            ),
            false
        ))
    }

    fn get_name(&self) -> &str { "modify" }
    fn get_description(&self) -> &str { "Modify currency name, state or currency code" }
    fn register(&self) -> Vec<CreateApplicationCommandOption> {
        vec![
            CreateApplicationCommandOption::default()
                .kind(CommandOptionType::SubCommand)
                .name("name")
                .description("Modify currency name")
                .create_sub_option(|option| {
                    option
                        .kind(CommandOptionType::String)
                        .name("code")
                        .description("Three-letter currency code to modify")
                        .min_length(3)
                        .max_length(3)
                        .required(true)
                })
                .create_sub_option(|option| {
                    option
                        .kind(CommandOptionType::String)
                        .name("name")
                        .description("New name of the currency")
                        .required(true)
                }).clone(),
            CreateApplicationCommandOption::default()
                .kind(CommandOptionType::SubCommand)
                .name("state")
                .description("Modify nation/state of origin of a currency")
                .create_sub_option(|option| {
                    option
                        .kind(CommandOptionType::String)
                        .name("code")
                        .description("Three-letter currency code to modify")
                        .min_length(3)
                        .max_length(3)
                        .required(true)
                })
                .create_sub_option(|option| {
                    option
                        .kind(CommandOptionType::String)
                        .name("state")
                        .description("New nation/state of the currency")
                        .required(true)
                }).clone(),
            CreateApplicationCommandOption::default()
                .kind(CommandOptionType::SubCommand)
                .name("code")
                .description("Modify three-letter currency code")
                .create_sub_option(|option| {
                    option
                        .kind(CommandOptionType::String)
                        .name("old_code")
                        .description("Old three-letter currency code")
                        .min_length(3)
                        .max_length(3)
                        .required(true)
                })
                .create_sub_option(|option| {
                    option
                        .kind(CommandOptionType::String)
                        .name("new_code")
                        .description("New three-letter currency code")
                        .min_length(3)
                        .max_length(3)
                        .required(true)
                }).clone()
        ]
    }
}

impl ModifyHandler {
    pub fn new() -> Self {
        ModifyHandler {}
    }

    fn parse_options(&self, options: &Vec<CommandDataOption>) -> Result<ModifyOptions, String> {

        let mut opts = ModifyOptions {
            code: None,
            name: None,
            state: None,
            old_code: None,
            new_code: None
        };

        for option in options {
            match option.name.as_str() {
                "code" => { if let Some(CommandDataOptionValue::String(c)) = option.resolved.clone() {
                    opts.code = Some(c);
                }},
                "old_code" => { if let Some(CommandDataOptionValue::String(c)) = option.resolved.clone() {
                    opts.old_code = Some(c);
                }},
                "new_code" => { if let Some(CommandDataOptionValue::String(c)) = option.resolved.clone() {
                    opts.new_code = Some(c);
                }},
                "state" => { if let Some(CommandDataOptionValue::String(s)) = option.resolved.clone() {
                    opts.state = Some(s);
                }},
                "name" => { if let Some(CommandDataOptionValue::String(n)) = option.resolved.clone() {
                    opts.name = Some(n);
                }},
                _ => {}
            }
        }

        Ok(opts)
    }
}
