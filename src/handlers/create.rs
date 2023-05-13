use crate::commands::manage::*;
use crate::commands::query::*;
use crate::types::*;
use crate::utils;
use crate::CommandResponseObject;
use async_trait::async_trait;
use serenity::builder::CreateApplicationCommandOption;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue
};

pub struct CreateHandler {}

#[async_trait]
impl ApplicationCommandHandler for CreateHandler {
    async fn handle_application_command(&mut self, data: &ApplicationCommandInteraction, _query_agent: &DBQueryAgent, manager: &DBManager) -> Result<CommandResponseObject, String> {
        let options = match utils::get_options(&data) {
            Ok(o) => o,
            Err(e) => return Err(format!("Error while getting options from command data: {e:?}"))
        };

        let (
            currency_code,
            currency_name,
            currency_state,
            initial_reserves,
            initial_circulation
        ) = match self.parse_options(&options) {
            Ok((a, b, c, d, e)) => (a, b, c, d, e),
            Err(e) => return Err(format!("Error while parsing options: {e:?}"))
        };

        let currency_data = match manager.add_currency(
            currency_code.clone(),
            currency_name.clone(),
            initial_circulation,
            initial_reserves,
            currency_state.clone(),
            data.user.name.clone()
        ).await {
            Ok(d) => d,
            Err(e) => return Err(format!("Error adding currency to database: {e:?}"))
        };

        Ok(CommandResponseObject::text(
            format!(
                    "{5} created new currency:\n> **{0}** (*{4}*)\n> Currency Code: `{1}`\n> Initial circulation: `{2}{1}`\n> Initial gold reserve: `{3} ingots`",
                    currency_data.currency_name,
                    currency_data.currency_code,
                    currency_data.circulation,
                    currency_data.reserves,
                    currency_data.state,
                    data.user
            ),
        ))
    }

    fn get_name(&self) -> &str { "create" }
    fn get_description(&self) -> &str { "Create a new currency" }
    fn get_option_kind(&self) -> CommandOptionType { CommandOptionType::SubCommand }
    fn register(&self) -> Vec<CreateApplicationCommandOption> {
        vec![
            CreateApplicationCommandOption::default()
                .kind(CommandOptionType::String)
                .name("code")
                .description("A three-letter currency code. This must be unique.")
                .min_length(3)
                .max_length(3)
                .required(true)
                .clone(),
            CreateApplicationCommandOption::default()
                .kind(CommandOptionType::String)
                .name("name")
                .description("The name of your new currency! This does *not* need to be unique")
                .required(true)
                .clone(),
            CreateApplicationCommandOption::default()
                .kind(CommandOptionType::String)
                .name("state")
                .description("The name of the nation or state in which this currency is based")
                .required(true)
                .clone(),
            CreateApplicationCommandOption::default()
                .kind(CommandOptionType::Integer)
                .name("initial_circulation")
                .description("The initial amount of your currency in circulation. Leave this blank if you're unsure")
                .clone(),
            CreateApplicationCommandOption::default()
                .kind(CommandOptionType::Integer)
                .name("initial_reserve")
                .description("The initial amount of gold in your federal reserve. Leave this blank if you're unsure.")
                .clone()
        ]
    }
}

impl CreateHandler {
    pub fn new() -> Self {
        CreateHandler {}
    }

    fn parse_options(&self, options: &Vec<CommandDataOption>) -> Result<(String, String, String, i64, i64), String> {
        let mut currency_code = None;
        let mut currency_name = None;
        let mut currency_state = None;
        let mut initial_reserves: i64 = 0;
        let mut initial_circulation: i64 = 0;

        for option in options {
            match option.name.as_str() {
                "code" => if let Some(CommandDataOptionValue::String(code)) = option.resolved.clone() { currency_code = Some(code) },
                "name" => if let Some(CommandDataOptionValue::String(name)) = option.resolved.clone() { currency_name = Some(name) },
                "state" => if let Some(CommandDataOptionValue::String(state)) = option.resolved.clone() { currency_state = Some(state) },
                "initial_circulation" => if let Some(CommandDataOptionValue::Integer(circulation)) = option.resolved.clone() {
                    if circulation >= 0 { initial_circulation = circulation }
                },
                "initial_reserve" => if let Some(CommandDataOptionValue::Integer(reserves)) = option.resolved.clone() {
                    if reserves >= 0 { initial_reserves = reserves }
                },
                _ => {}
            }
        }

        if currency_code == None { return Err("Error: no currency code specified".into()) }
        if currency_name == None { return Err("Error: no currency name specified".into()) }
        if currency_state == None { return Err("Error: no currency state specified".into()) }

        Ok((
            currency_code.unwrap(),
            currency_name.unwrap(),
            currency_state.unwrap(),
            initial_reserves,
            initial_circulation
        ))
    }
}
