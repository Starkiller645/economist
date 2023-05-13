use crate::commands::manage::*;
use crate::commands::query::*;
use crate::types::*;
use crate::utils;
use crate::CommandResponseObject;
use async_trait::async_trait;
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::builder::{CreateComponents, CreateApplicationCommandOption};
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::application::component::ButtonStyle;
use serenity::model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
};

pub struct DeleteHandler {
    currency_code: String,
    currency_name: String
}

#[async_trait]
impl ApplicationCommandHandler for DeleteHandler {
    async fn handle_application_command(&mut self, data: &ApplicationCommandInteraction, query_agent: &DBQueryAgent, _manager: &DBManager) -> Result<CommandResponseObject, String> {

        let options = match utils::get_options(&data) {
            Ok(o) => o,
            Err(e) => return Err(format!("Error while getting options from command data: {e:?}"))
        };

        let currency_code = match self.parse_options(&options) {
            Ok(c) => c,
            Err(e) => return Err(format!("Error while parsing options: {e:?}"))
        };

        let currency_data = match query_agent.get_currency_data(currency_code.clone()).await {
            Ok(data) => data,
            Err(_e) => {
                return Err(format!("Error: could not find the currency code `{currency_code}`"))
            }
        };

        let user_name = data.user.name.clone();
        if user_name != currency_data.owner {
            return Err(format!("Error: you are not the owner of this currency, and therefore cannot modify it"))
        };

        let components = CreateComponents::default()
            .create_action_row(|action_row| {
                action_row
                    .create_button(|button| {
                        button
                            .label("Confirm")
                            .custom_id("delete-confirm")
                                .style(ButtonStyle::Danger)
                        })
                        .create_button(|button| {
                            button
                                .label("Cancel")
                                .custom_id("delete-cancel")
                                .style(ButtonStyle::Primary)
                        })
                }).clone();

        self.currency_code = currency_data.currency_code.clone();
        self.currency_name = currency_data.currency_name.clone();

        Ok(CommandResponseObject::interactive(
            components,
            format!("Confirm you really want to delete the currency **{}** `{}`?\n*This is not reversible*", currency_data.currency_name, currency_data.currency_code),
            true
        ))

    }

    fn get_name(&self) -> &str { "delete" }
    fn get_description(&self) -> &str { "Delete a currency from the database" }
    fn get_option_kind(&self) -> CommandOptionType { CommandOptionType::SubCommand }
    fn register(&self) -> Vec<CreateApplicationCommandOption> {
        vec![
            CreateApplicationCommandOption::default()
                .kind(CommandOptionType::String)
                .name("code")
                .description("The three-letter currency code to delete.")
                .min_length(3)
                .max_length(3)
                .required(true)
                .clone()
        ]
    }
}

#[async_trait]
impl InteractionResponseHandler for DeleteHandler {
    async fn handle_interaction_response(&self, data: &MessageComponentInteraction, _query_agent: &DBQueryAgent, manager: &DBManager) -> Result<CommandResponseObject, String> {

        let confirm = match data.data.custom_id.as_str() {
            "delete-confirm" => true,
            _ => false
        };

        if confirm {
            match manager.remove_currency(self.currency_code.clone()).await {
                Ok(_) => Ok(CommandResponseObject::interactive_with_feedback(
                    CreateComponents::default(),
                    format!(
                        "Successfully deleted currency **{}** `{}`",
                        self.currency_name, self.currency_code
                    ), 
                    format!("{} deleted currency **{}** `{}`", 
                        data.user,
                        self.currency_name, 
                        self.currency_code
                    ), 
                    true
                )),
                Err(e) => Err(format!("Error removing currency from database: {e:?}"))
            }
        } else {
            Ok(CommandResponseObject::interactive_with_feedback(
                CreateComponents::default(),
                format!("Will not delete currency **{}** `{}`", self.currency_name, self.currency_code),
                "",
                true
            ))
        }
    }

    fn get_pattern(&self) -> Vec<&str> {
        vec!["delete-confirm", "delete-cancel"]
    }
}

impl DeleteHandler {
    pub fn new() -> Self {
        DeleteHandler {
            currency_code: String::new(),
            currency_name: String::new()
        }
    }

    fn parse_options(&self, options: &Vec<CommandDataOption>) -> Result<String, String> {
        let mut currency_code = None;

        for option in options {
            match option.name.as_str() {
                "code" => if let Some(CommandDataOptionValue::String(code)) = option.resolved.clone() {
                    currency_code = Some(code);
                },
                _ => {}
            }
        }

        if currency_code == None { return Err("Error: no currency code specified".into()) }

        Ok(currency_code.unwrap())
    }
}
