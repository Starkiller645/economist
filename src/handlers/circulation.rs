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

pub struct CirculationHandler {
    transaction_code: String,
    transaction_amount: i64,
    transaction_initiator: User
}

#[async_trait]
impl ApplicationCommandHandler for CirculationHandler {
    async fn handle_application_command(&mut self, data: &ApplicationCommandInteraction, query_agent: &DBQueryAgent, _manager: &DBManager) -> Result<CommandResponseObject, String> {
        info!("Handling command from `{}`", self.get_name());
        let cmd = match data.data.options.get(0) {
            Some(a) => a,
            None => return Err("Couldn't get subcommand data".into())
        };

        let action = match cmd.options.get(0) {
            Some(a) => a,
            None => return Err("Couldn't get subcommand options".into())
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

        info!("Parsing options");
        let (amount, currency_code) = match self.parse_options(&options, add) {
            Ok((a, b)) => (a.clone(), b.clone()),
            Err(e) => {
                return match e {
                    "add" => Err("Can't use negative values with `/currency circulation add`. Please use `/currency circulation remove` instead.".into()),
                    "remove" => Err("Can't use negative values with `/currency circulation remove`. Please use `/currency circulation add` instead.".into()),
                    _ => Err("An unknown error occured while parsing command arguments".into())
                };
            }
        };

        self.transaction_code = currency_code.clone();
        self.transaction_amount = amount;
        self.transaction_initiator = data.user.clone();

        info!("Checking currency data");
        match query_agent.get_currency_data(currency_code.clone()).await {
            Ok(data) => Ok(self.generate_command_response(data, amount, add)),
            Err(e) => Err(format!("An error occured while performing a database lookup: {e:?}"))
        }
    }
    fn get_name(&self) -> &str { "circulation" }
    fn get_description(&self) -> &str { "Manage circulation amounts of a currency" }
    fn register(&self) -> Vec<CreateApplicationCommandOption> {
        vec![
        CreateApplicationCommandOption::default()
            .kind(CommandOptionType::SubCommand)
            .name("add")
            .description("Put money into circulation")
            .create_sub_option(|option| {
                option
                    .kind(CommandOptionType::Integer)
                    .name("amount")
                    .description("The amount of money to put into circulation")
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
            .description("Remove money from circulation")
            .create_sub_option(|option| {
                option
                    .kind(CommandOptionType::Integer)
                    .name("amount")
                    .description("The amount of money to remove from circulation")
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
impl InteractionResponseHandler for CirculationHandler {
    async fn handle_interaction_response(&self, data: &MessageComponentInteraction, query_agent: &DBQueryAgent, manager: &DBManager) -> Result<CommandResponseObject, String> {
        match data.data.custom_id.as_str() {
            "circulation-transaction-confirm" => {
                info!("Transaction details: code: `{}`, amount: `{}`, initiator: `{}`", self.transaction_code.clone(), self.transaction_amount, self.transaction_initiator.name.clone());
                let transaction_response = match manager.circulation_modify(self.transaction_code.clone(), self.transaction_amount, self.transaction_initiator.name.clone()).await {
                    Ok(data) => data,
                    Err(e) => return Err(format!("Error while completing circulation transaction: `{e:?}`"))
                };

                let currency_data = match query_agent.get_currency_data(self.transaction_code.clone()).await {
                            Ok(data) => data,
                            Err(e) => return Ok(CommandResponseObject::interactive_with_feedback(CreateComponents::default(), format!("Error while completing currency balance check: `{e:?}`"), "", true))
                        };

                let feedback = format!("Successfully completed currency circulation transaction!");
                let broadcast = format!("{0} made a currency circulation transaction:\n> Currency: **{1}** `{2}`\n> Nation/State: *{6}*\n> Amount: `{3}{2}`\n> New balance: `{4}{2}`\n> Transaction ID: `#{5:0>5}`", data.user, currency_data.currency_name, self.transaction_code, self.transaction_amount, currency_data.circulation, transaction_response.transaction_id, currency_data.state);
                Ok(CommandResponseObject::interactive_with_feedback(CreateComponents::default(), feedback, broadcast, true))
            }
            "circulation-transaction-cancel" => {
                Ok(CommandResponseObject::interactive_with_feedback(CreateComponents::default(), "Cancelled transaction. No records were updated.", "", true))
            },
            _ => {Ok(CommandResponseObject::text(""))}
        }
    }

    fn get_pattern(&self) -> Vec<&str> {
        vec!["circulation-transaction-confirm","circulation-transaction-cancel"]
    }
}

impl CirculationHandler {
    pub fn new() -> Self {
        CirculationHandler {
            transaction_code: String::new(),
            transaction_amount: 0,
            transaction_initiator: User::default()
        }
    }

    fn parse_options(&self, options: &Vec<CommandDataOption>, add: bool) -> Result<(i64, String), &str> {
        let mut amount: i64 = 0;
        let mut currency_code = String::new();

        for option in options {
            info!("Checking option: {option:?}");
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

    fn generate_command_response(&self, data: CurrencyData, amount: i64, add: bool) -> CommandResponseObject {
        let new_circulation = data.circulation + amount;
        let mut warning = "";
        let mut confirm_style = ButtonStyle::Primary;
        let mut cancel_style = ButtonStyle::Secondary;

        if !add {
            confirm_style = ButtonStyle::Danger;
            cancel_style = ButtonStyle::Primary;
            warning = "\n**Warning**: You must only use this command if you are *certain* that you have removed the correct amount from circulation by repossessing and destroying it.\nUsing this command without doing so could result in prosecution by the Gold Standard organisation, as it will increase your currency's value illegally!";
        }

        let components = CreateComponents::default()
            .create_action_row(|action_row| {
                action_row
                    .create_button(|button| {
                        button
                            .label("Confirm")
                            .style(confirm_style)
                            .custom_id("circulation-transaction-confirm")
                    })
                    .create_button(|button| {
                        button
                            .label("Cancel")
                            .style(cancel_style)
                            .custom_id("circulation-transaction-cancel")
                    })
            }).clone();

        CommandResponseObject::interactive(
            components,
            format!("**Review currency circulation transaction**\n> Currency: **{0}** `{1}`\n> Nation/State: *{2}*\n> Amount: `{amount}{1}`\n> New balance: `{3}{1}`{4}", data.currency_name, data.currency_code, data.state, new_circulation, warning),
            true
        )
    }
}
