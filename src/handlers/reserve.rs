use crate::types::*;
use crate::CommandResponseObject;
use async_trait::async_trait;
use serenity::model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOptionValue,
    CommandDataOption
};
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::model::application::component::ButtonStyle;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::user::User;
use serenity::builder::{CreateComponents, CreateApplicationCommandOption};
use crate::commands::query::*;
use crate::commands::manage::*;
use crate::utils;
use tracing::info;

pub struct ReserveHandler {
    transaction_code: String,
    transaction_amount: i64,
    transaction_initiator: User
}

#[async_trait]
impl ApplicationCommandHandler for ReserveHandler {
    async fn handle_application_command(&mut self, data: &ApplicationCommandInteraction, query_agent: &DBQueryAgent, _manager: &DBManager) -> Result<CommandResponseObject, String> {
        info!("Handling command from `{}`", self.get_name());
        let cmd = match data.data.options.get(0) {
            Some(a) => a,
            None => return Err("Couldn't get subcommand data".into())
        };

        let action = match cmd.options.get(0) {
            Some(a) => a,
            None => return Err("Couldn't get options from application data".into())
        };

        let add = match action.name.as_str() {
            "add" => true,
            "remove" => false,
            _ => false,
        };

        let options = match utils::get_options(data) {
            Ok(o) => o,
            Err(e) => return Err(format!("Error while parsing options: {e:?}"))
        };

        let (amount, currency_code) = match self.parse_options(&options, add) {
            Ok((a, b)) => (a.clone(), b.clone()),
            Err(e) => {
                return match e {
                    "add" => Err("Can't use negative values with `/currency reserve add`. Please use `/currency reserve remove` instead.".into()),
                    "remove" => Err("Can't use negative values with `/currency reserve remove`. Please use `/currency reserve add` instead.".into()),
                    _ => Err("An unknown error occured while parsing command arguments".into())
                };
            }
        };

        self.transaction_code = currency_code.clone();
        self.transaction_amount = amount;
        self.transaction_initiator = data.user.clone();

        match query_agent.get_currency_data(currency_code.clone()).await {
            Ok(currency_data) => if data.user.name == currency_data.owner {
                Ok(self.generate_command_response(currency_data, amount))
            } else {
                Err("Error: you are not the owner of this currency, and therefore cannot modify it".into())
            },
            Err(e) => Err(format!("An error occured while performing a database lookup: {e:?}"))
        }
    }
    fn get_name(&self) -> &str { "reserve" }
    fn get_description(&self) -> &str { "Manage gold reserves of a currency" }
    fn register(&self) -> Vec<CreateApplicationCommandOption> {
        vec![
        CreateApplicationCommandOption::default()
            .kind(CommandOptionType::SubCommand)
            .name("add")
            .description("Add gold to federal reserves")
            .create_sub_option(|option| {
                option
                    .kind(CommandOptionType::Integer)
                    .name("amount")
                    .description("The amount of gold to add")
                    .required(true)
            })
            .create_sub_option(|option| {
                option
                    .kind(CommandOptionType::String) 
                    .name("code")
                    .min_length(3)
                    .max_length(3)
                    .description("The three-letter code of the target currency")
                    .required(true)
            }).clone(),
        CreateApplicationCommandOption::default()
            .kind(CommandOptionType::SubCommand)
            .name("remove")
            .description("Remove gold from federal reserves")
            .create_sub_option(|option| {
                option
                    .kind(CommandOptionType::Integer)
                    .name("amount")
                    .description("The amount of gold to remove")
                    .required(true)
            })
            .create_sub_option(|option| {
                option
                    .kind(CommandOptionType::String) 
                    .name("code")
                    .min_length(3)
                    .max_length(3)
                    .description("The three-letter code of the target currency")
                    .required(true)
            }).clone()
        ]
    }
}

#[async_trait]
impl InteractionResponseHandler for ReserveHandler {
    async fn handle_interaction_response(&self, data: &MessageComponentInteraction, query_agent: &DBQueryAgent, manager: &DBManager) -> Result<CommandResponseObject, String> {
        info!("Transaction details: code: `{}`, amount: `{}`, initiator: `{}`", self.transaction_code.clone(), self.transaction_amount, self.transaction_initiator.name.clone());
        match data.data.custom_id.as_str() {
            "reserve-transaction-confirm" => {
                let transaction_response = match manager.reserve_modify(self.transaction_code.clone(), self.transaction_amount, self.transaction_initiator.name.clone()).await {
                    Ok(data) => data,
                    Err(e) => return Err(format!("Error while completing reserve transaction: `{e:?}`"))
                };

                let currency_data = match query_agent.get_currency_data(self.transaction_code.clone()).await {
                            Ok(data) => data,
                            Err(e) => return Ok(CommandResponseObject::interactive_with_feedback(CreateComponents::default(), format!("Error while completing currency reserve check: `{e:?}`"), "", true))
                        };

                let feedback = format!("Successfully completed gold reserve transaction!");
                let broadcast = format!("{0} made a gold reserve transaction:\n> Currency: **{1}** `{2}`\n> Nation/State: *{6}*\n> Amount: `{3} ingots`\n> New balance: `{4} ingots`\n> Transaction ID: `#{5:0>5}`", data.user, currency_data.currency_name, self.transaction_code, self.transaction_amount, currency_data.reserves, transaction_response.transaction_id, currency_data.state);

                Ok(CommandResponseObject::interactive_with_feedback(CreateComponents::default(), feedback, broadcast, true))
            },
            "reserve-transaction-cancel" => {
                Ok(CommandResponseObject::interactive_with_feedback(CreateComponents::default(), "Cancelled transaction. No records were updated.", "", true))
            },
            _ => {Ok(CommandResponseObject::text(""))}

        }
    }
    fn get_pattern(&self) -> Vec<&str> {
        vec!["reserve-transaction-confirm","reserve-transaction-cancel"]
    }
}

impl ReserveHandler {
    pub fn new() -> Self {
        ReserveHandler {
            transaction_code: String::new(),
            transaction_amount: 0,
            transaction_initiator: User::default()
        }
    }

    fn parse_options(&self, options: &Vec<CommandDataOption>, add: bool) -> Result<(i64, String), &str> {
        let mut amount: i64 = 0;
        let mut currency_code = String::new();

        for option in options {
            match option.name.as_str() {
                "code" => {
                    if let Some(CommandDataOptionValue::String(code)) = option.resolved.clone() {
                        currency_code = code;
                    }
                },
                "amount" => {
                    if let Some(CommandDataOptionValue::Integer(transaction_amount)) = option.resolved {
                        if transaction_amount < 0 {
                            if add {
                                return Err("add")
                            } else {
                                return Err("remove")
                            }
                        }
                        amount = if add {
                            transaction_amount
                        } else {
                            -transaction_amount
                        };
                    }
                }
                _ => {}
            }
        }

        Ok((amount, currency_code))
    }

    fn generate_command_response(&self, data: CurrencyData, amount: i64) -> CommandResponseObject {
        let new_reserves = data.reserves + amount;
        
        let components = CreateComponents::default()
            .create_action_row(|action_row| {
                action_row
                    .create_button(|button| {
                        button
                            .label("Confirm")
                            .style(ButtonStyle::Primary)
                            .custom_id("reserve-transaction-confirm")
                    })
                    .create_button(|button| {
                        button
                            .label("Cancel")
                            .style(ButtonStyle::Secondary)
                            .custom_id("reserve-transaction-cancel")
                    })
            }).clone();

        CommandResponseObject::interactive(
            components,
            format!("**Review gold reserve transaction**\n> Currency: **{0}** `{1}`\n> Nation/State: *{2}*\n> Amount: `{amount} ingots`\n> New balance: `{3} ingots`", data.currency_name, data.currency_code, data.state, new_reserves),
            true
        )
    }
}
