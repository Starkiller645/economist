use crate::CommandResponseObject;
use crate::commands::manage::*;
use crate::commands::query::*;
use serenity::builder::{
    CreateApplicationCommand, CreateComponents,
};
use serenity::model::application::component::ButtonStyle;
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOptionValue,
};
use chrono::DateTime;
use chrono::offset::Utc;
use tracing::info;
use crate::types::*;

pub struct CurrencyHandler {
    manager: DBManager,
    query_agent: DBQueryAgent
}

impl CurrencyHandler {
    pub fn new(manager: DBManager, query_agent: DBQueryAgent) -> Self {
        CurrencyHandler {
            manager,
            query_agent
        }
    }

    pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        command
            .name("currency")
            .description("Manage currencies and their circulation/reserve levels")
            .create_option(|option| {
                option
                    .kind(CommandOptionType::SubCommand)
                    .name("create")
                    .description("Create a new currency")
                    .create_sub_option(|option| {
                        option
                            .kind(CommandOptionType::String)
                            .name("code")
                            .description("A three-letter currency code. This must be unique.")
                            .min_length(3)
                            .max_length(3)
                            .required(true)
                    })
                    .create_sub_option(|option| {
                        option
                            .kind(CommandOptionType::String)
                            .name("name")
                            .description("The name of your new currency! This does *not* need to be unique.")
                            .required(true)
                    })
                    .create_sub_option(|option| {
                        option
                            .kind(CommandOptionType::String)
                            .name("state")
                            .description("The name of the nation or state in which this currency is based.")
                            .required(true)
                    })
                    .create_sub_option(|option| {
                        option
                            .kind(CommandOptionType::Integer)
                            .name("initial_circulation")
                            .description("The initial amount of your currency in circulation. Leave this blank if you're unsure.")
                    })
                    .create_sub_option(|option| {
                        option
                            .kind(CommandOptionType::Integer)
                            .name("initial_reserve")
                            .description("The initial amount of gold in your federal reserve. Leave this blank if you're unsure.")
                    })
            })
            .create_option(|option| {
                option
                    .kind(CommandOptionType::SubCommand)
                    .name("delete")
                    .description("Delete a currency from the database")
                    .create_sub_option(|option| {
                        option
                            .kind(CommandOptionType::String)
                            .name("code")
                            .description("The three-letter currency code to delete.")
                            .min_length(3)
                            .max_length(3)
                            .required(true)
                    })
            })
            .create_option(|option| {
                option
                    .kind(CommandOptionType::SubCommandGroup)
                    .name("reserve")
                    .description("Manage gold reserves of a currency")
                    .create_sub_option(|option| {
                        option
                            .kind(CommandOptionType::SubCommand)
                            .name("add")
                            .description("Add gold to a currency's reserves")
                            .create_sub_option(|option| {
                                option
                                    .kind(CommandOptionType::Integer)
                                    .name("amount")
                                    .description("The amount of gold to add to the reserves")
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
                            })
                    })
                    .create_sub_option(|option| {
                        option
                            .kind(CommandOptionType::SubCommand)
                            .name("remove")
                            .description("Remove gold from a currency's reserves")
                            .create_sub_option(|option| {
                                option
                                    .kind(CommandOptionType::Integer)
                                    .name("amount")
                                    .description("The amount of gold to remove from the reserves")
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
                            })
                    })
            })
            .create_option(|option| {
                option
                    .kind(CommandOptionType::SubCommandGroup)
                    .name("circulation")
                    .description("Manage circulation amounts of a currency")
                    .create_sub_option(|option| {
                        option
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
                            })
                    })
                    .create_sub_option(|option| {
                        option
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
                            })
                    })
            })
            .create_option(|option| {
                option
                    .kind(CommandOptionType::SubCommandGroup)
                    .name("modify")
                    .description("Modify currency name, state, or currency code.")
                    .create_sub_option(|option| {
                        option
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
                            })
                    })
                    .create_sub_option(|option| {
                        option
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
                            })
                    })
                    .create_sub_option(|option| {
                        option
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
                            })
                    })
            })
            .create_option(|option| {
                option
                    .kind(CommandOptionType::SubCommand)
                    .name("recreate-database")
                    .description("Recreate the entire currency database, starting from scratch. DANGER, THIS IS NOT REVERSIBLE!!!")
            })
            .create_option(|option| {
                option
                    .kind(CommandOptionType::SubCommand) 
                    .name("list")
                    .description("List currencies in circulation, optionally specifying a number")
                    .create_sub_option(|option| {
                        option
                            .kind(CommandOptionType::Integer)
                            .name("number")
                            .description("Number of currencies to list")
                    })
                    .create_sub_option(|option| {
                        option
                            .kind(CommandOptionType::String)
                            .name("sort")
                            .description("Attribute to sort currency list by")
                            .add_string_choice("Name", "name")
                            .add_string_choice("Nation/State", "state")
                            .add_string_choice("Currency Code", "code")
                            .add_string_choice("Gold Reserves", "reserves")
                            .add_string_choice("Circulation", "circulation")
                            .add_string_choice("Value", "value")
                    })
            })
            .create_option(|option| {
                option
                    .kind(CommandOptionType::SubCommand)
                    .name("view")
                    .description("View detailed information about a currency")
                    .create_sub_option(|option| {
                        option
                            .kind(CommandOptionType::String)
                            .name("code")
                            .description("Three-letter currency code to view")
                            .max_length(3)
                            .min_length(3)
                            .required(true)
                    })
            })
    }

    pub async fn run(&self, data: &ApplicationCommandInteraction, custom_data: &std::sync::Mutex<std::collections::HashMap<String, String>>) -> CommandResponseObject {

        let subcommand_data = match data.data
            .options.get(0) {
                Some(data) => data,
                None => return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: Economist Bot encountered an error processing a command. A debug dump of the command interaction is below:\n{data:?}"), false)
            };
        
        let mut currency_name = String::new();
        let mut currency_code = String::new();
        let mut circulation: i64 = 0;
        let mut gold_reserve: i64 = 0;
        let mut state = String::new();

        for option in subcommand_data.options.clone() {
            match option.name.as_str() {
                "code" => { if let Some(CommandDataOptionValue::String(code)) = option.resolved {
                    currency_code = code;
                }},
                "name" => { if let Some(CommandDataOptionValue::String(name)) = option.resolved {
                    currency_name = name;
                }},
                "initial_circulation" => { if let Some(CommandDataOptionValue::Integer(initial_circulation)) = option.resolved {
                    circulation = initial_circulation;
                }},
                "initial_reserve" => { if let Some(CommandDataOptionValue::Integer(initial_reserve)) = option.resolved{
                    gold_reserve = initial_reserve;
                }},
                "state" => { if let Some(CommandDataOptionValue::String(state_name)) = option.resolved {
                    state = state_name;
                }}
                _ => {}
            }
        }

        match subcommand_data.name.as_str() {
            "delete" => {
                let currency_name: String;
                {
                    let mut custom_data = custom_data.lock().unwrap();
                    custom_data.insert("currency_code".into(), currency_code.clone());
                }

                let currency_data = match self.query_agent.get_currency_data(currency_code.clone()).await {
                    Ok(data) => data,
                    Err(_e) => {
                        return CommandResponseObject::text(format!("Error deleting currency: could not find the currency code `{currency_code}`"))
                    }
                };
                currency_name = currency_data.currency_name;
                {
                    let mut custom_data = custom_data.lock().unwrap();
                    custom_data.insert("currency_name".into(), currency_name.clone());
                }
            CommandResponseObject::interactive(
                CreateComponents::default()
                    .create_action_row(|action_row| {
                        action_row
                            .create_button(|button| {
                                button
                                    .label("Confirm")
                                    .custom_id("button-delete-confirm")
                                        .style(ButtonStyle::Danger)
                                })
                                .create_button(|button| {
                                    button
                                        .label("Cancel")
                                        .custom_id("button-delete-cancel")
                                        .style(ButtonStyle::Primary)
                                })
                            /*action_row.create_input_text(|input| {
                                input
                                    .label("Hello")
                                    .custom_id("hello-input")
                                    .style(InputTextStyle::Short)
                            })*/
                        })
                        .clone(),
                format!("Confirm you really want to delete the currency **{currency_name}** `{currency_code}`?\n*This is not reversible*"),
                true
                )
            }

            "create" => {
                match self.manager.add_currency(currency_code.clone(), currency_name.clone(), circulation, gold_reserve, state).await
                {
                    Ok(currency_data) => {
                        {
                            let mut custom_data = custom_data.lock().unwrap();
                            custom_data.insert("currency_code".into(), currency_code.clone());
                            custom_data.insert("currency_name".into(), currency_name.clone());
                        }
                        CommandResponseObject::text(
                            format!(
                                    "{5} created new currency:\n> **{0}** (*{4}*)\n> Currency Code: `{1}`\n> Initial circulation: `{2}{1}`\n> Initial gold reserve: `{3} ingots`",
                                    currency_data.currency_name,
                                    currency_data.currency_code,
                                    currency_data.circulation,
                                    currency_data.reserves,
                                    currency_data.state,
                                    data.user
                            ),
                        )
                    },
                    Err(e) => {
                        let error_message = format!("{:?}", e);
                        if let Some(error) = e.into_database_error() {
                            match error.downcast_ref::<sqlx::postgres::PgDatabaseError>().code() {
                                "23505" => CommandResponseObject::interactive(
                                    CreateComponents::default(),
                                    format!(
                                        "Error creating currency **{currency_name}**:\nThe currency code `{currency_code}` is already in use, please choose another one!"
                                    ),
                                    true
                                ),
                                err => CommandResponseObject::text(
                                    format!(
                                        "Error creating currency **{currency_name}**:\n```{:?}```",
                                        err
                                    )
                                )
                            }
                        } else {
                            CommandResponseObject::text(
                                format!(
                                    "Error creating currency **{currency_name}**:\n```{:?}```",
                                    error_message
                                )
                            )
                        }
                    }
                }
            }
            "reserve" => {
                let action = match subcommand_data.options.get(0) {
                    Some(a) => a,
                    None => return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: Economist Bot encountered an error processing a command. A debug dump of the command interaction is below:\n{data:?}"), true)
                };

                let mut amount: i64 = 0;
                let mut currency_code = String::new();

                match action.name.as_str() {
                    "add" => {
                        for option in action.options.clone() {
                            match option.name.as_str() {
                                "code" => { if let Some(CommandDataOptionValue::String(code)) = option.resolved {
                                    currency_code = code;
                                }},
                                "amount" => { if let Some(CommandDataOptionValue::Integer(transaction_amount)) = option.resolved {
                                    if amount < 0 {
                                        return CommandResponseObject::interactive(CreateComponents::default(), "Error: you can't use negative values with `/currency reserve add`. Please use `/currency reserve remove` instead.", true)
                                    } else {
                                        amount = transaction_amount
                                    }
                                }},
                                _ => {}
                            }
                        }
                    },
                    "remove" => {
                        for option in action.options.clone() {
                            match option.name.as_str() {
                                "code" => { if let Some(CommandDataOptionValue::String(code)) = option.resolved {
                                    currency_code = code;
                                }},
                                "amount" => { if let Some(CommandDataOptionValue::Integer(transaction_amount)) = option.resolved {
                                    if amount < 0 {
                                        return CommandResponseObject::interactive(CreateComponents::default(), "Error: you can't use negative values with `/currency reserve remove` to add more money. That's silly. Please use `/currency reserve add` instead.", true)
                                    } else {
                                        amount = -transaction_amount
                                    }
                                }},
                                _ => {}
                            }
                        }
                    }
                    _ => return CommandResponseObject::text("")
                };

                {
                    let mut c_data = custom_data.lock().unwrap();
                    c_data.insert("transaction_code".into(), currency_code.clone());
                    c_data.insert("transaction_amount".into(), format!("{}", amount));
                    c_data.insert("transaction_initiator".into(), format!("{}", data.user));
                }

                match self.query_agent.get_currency_data(currency_code.clone()).await {
                    Ok(data) => {
                        let new_reserves = data.reserves + amount;
                        CommandResponseObject::interactive(
                            CreateComponents::default()
                                .create_action_row(|action_row| {
                                    action_row
                                        .create_button(|button| {
                                            button
                                                .label("Confirm")
                                                .style(ButtonStyle::Primary)
                                                .custom_id("gold-transaction-confirm")
                                        })
                                        .create_button(|button| {
                                            button
                                                .label("Cancel")
                                                .style(ButtonStyle::Secondary)
                                                .custom_id("gold-transaction-cancel")
                                        })
                                }).clone(),
                            format!("**Review gold reserve transaction**\n> Currency: **{0}** `{1}`\n> Nation/State: *{2}*\n> Amount: `{amount} ingots`\n> New balance: `{3} ingots`", data.currency_name, data.currency_code, data.state, new_reserves),
                            true
                        )
                    },
                    Err(e) => {
                        return match e {
                            sqlx::Error::RowNotFound => CommandResponseObject::interactive(CreateComponents::default(), format!("Error: the currency code `{}` was not found. Check your spelling and try again.", currency_code), true),
                            _ => CommandResponseObject::interactive(CreateComponents::default(), format!("Error: SQLx Error\n`{:?}`", e), true)
                        }
                    }
                }
            },
			"circulation" => {
                let action = match subcommand_data.options.get(0) {
                    Some(a) => a,
                    None => return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: Economist Bot encountered an error processing a command. A debug dump of the command interaction is below:\n```{data:?}```"), true)
                };

                let mut amount: i64 = 0;
                let mut currency_code = String::new();

                match action.name.as_str() {
                    "add" => {
                        for option in action.options.clone() {
                            match option.name.as_str() {
                                "code" => { if let Some(CommandDataOptionValue::String(code)) = option.resolved {
                                    currency_code = code;
                                }},
                                "amount" => { if let Some(CommandDataOptionValue::Integer(transaction_amount)) = option.resolved {
                                    if amount < 0 {
                                        return CommandResponseObject::interactive(CreateComponents::default(), "Error: you can't use negative values with `/currency circulation add`. Please use `/currency circulation remove` instead.", true)
                                    } else {
                                        amount = transaction_amount
                                    }
                                }},
                                _ => {}
                            }
                        }
                    },
                    "remove" => {
                        for option in action.options.clone() {
                            match option.name.as_str() {
                                "code" => { if let Some(CommandDataOptionValue::String(code)) = option.resolved {
                                    currency_code = code;
                                }},
                                "amount" => { if let Some(CommandDataOptionValue::Integer(transaction_amount)) = option.resolved {
                                    if amount < 0 {
                                        return CommandResponseObject::interactive(CreateComponents::default(), "Error: you can't use negative values with `/currency circulation remove` to add more money. That's silly. Please use `/currency circulation add` instead.", true)
                                    } else {
                                        amount = -transaction_amount
                                    }
                                }},
                                _ => {}
                            }
                        }
                    }
                    _ => return CommandResponseObject::text("")
                };

                {
                    let mut c_data = custom_data.lock().unwrap();
                    c_data.insert("transaction_code".into(), currency_code.clone());
                    c_data.insert("transaction_amount".into(), format!("{}", amount));
                    c_data.insert("transaction_initiator".into(), format!("{}", data.user));
                }

                match self.query_agent.get_currency_data(currency_code.clone()).await {
                    Ok(data) => {
                        let new_circulation = data.circulation + amount;
						let mut warning = "";
						let mut confirm_style = ButtonStyle::Primary;
						let mut cancel_style = ButtonStyle::Secondary;
						if action.name == "remove" {
							confirm_style = ButtonStyle::Danger;
							cancel_style = ButtonStyle::Primary;
							warning = "\n**Warning**: You must only use this command if you are *certain* that you have removed the correct amount from circulation by repossessing and destroying it.\nUsing this command without doing so could result in prosecution by the Gold Standard organisation, as it will increase your currency's value illegally!"
						}

						let components = CreateComponents::default()
                                .create_action_row(|action_row| {
                                    action_row
                                        .create_button(|button| {
                                            button
                                                .label("Confirm")
                                                .style(confirm_style)
                                                .custom_id("currency-transaction-confirm")
                                        })
                                        .create_button(|button| {
                                            button
                                                .label("Cancel")
                                                .style(cancel_style)
                                                .custom_id("currency-transaction-cancel")
                                        })
                                }).clone();
						/*if action.name == "remove" {
							components = components
								.create_action_row(|action_row| {
									action_row
										.create_input_text(|input_text| {
											input_text
												.required(true)
												.placeholder("Type `Confirm Transaction` here to confirm you've understood the warnings")
												.custom_id("currency-transaction-remove-confirm")
												.style(InputTextStyle::Short)
                                                .label("Confirm Transaction")
										})
								}).clone()
						}*/

                        CommandResponseObject::interactive(
                            components,
                            format!("**Review currency circulation transaction**\n> Currency: **{0}** `{1}`\n> Nation/State: *{2}*\n> Amount: `{amount}{1}`\n> New balance: `{3}{1}`{4}", data.currency_name, data.currency_code, data.state, new_circulation, warning),
                            true
                        )
                    },
                    Err(e) => {
                        return match e {
                            sqlx::Error::RowNotFound => CommandResponseObject::interactive(CreateComponents::default(), format!("Error: the currency code `{}` was not found. Check your spelling and try again.", currency_code), true),
                            _ => CommandResponseObject::interactive(CreateComponents::default(), format!("Error: SQLx Error\n`{:?}`", e), true)
                        }
                    }
                }
            },
            "modify" => {
                let action = match subcommand_data.options.get(0) {
                    Some(a) => a,
                    None => return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: Economist Bot encountered an error processing a command. A debug dump of the command interaction is below:\n```{data:?}```"), true)
                };

                let mut code = None;
                let mut state = None;
                let mut name = None;
                let mut old_code = None;
                let mut new_code = None;

                for option in action.options.clone() {
                    match option.name.as_str() {
                        "code" => { if let Some(CommandDataOptionValue::String(c)) = option.resolved {
                            code = Some(c);
                        }},
                        "old_code" => { if let Some(CommandDataOptionValue::String(c)) = option.resolved {
                            old_code = Some(c);
                        }},
                        "new_code" => { if let Some(CommandDataOptionValue::String(c)) = option.resolved {
                            new_code = Some(c);
                        }},
                        "state" => { if let Some(CommandDataOptionValue::String(s)) = option.resolved {
                            state = Some(s);
                        }},
                        "name" => { if let Some(CommandDataOptionValue::String(n)) = option.resolved {
                            name = Some(n);
                        }},
                        _ => {}
                    }
                }

                let mut final_data: Result<CurrencyData, sqlx::Error> = Err(sqlx::Error::Protocol("Error in command args".into()));
                match action.name.as_str() {
                    "code" => {
                        if let Some(c) = old_code {
                            if let Some(nc) = new_code {
                                final_data = self.manager.modify_currency_meta(c, ModifyMetaType::Code, nc).await;
                            }
                        }
                    },
                    "state" => {
                        if let Some(c) = code {
                            if let Some(s) = state {
                                final_data = self.manager.modify_currency_meta(c, ModifyMetaType::State, s).await;
                            }
                        }
                    },
                    "name" => {
                        if let Some(c) = code {
                            if let Some(n) = name {
                                final_data = self.manager.modify_currency_meta(c, ModifyMetaType::Name, n).await;
                            }
                        }
                    }
                    _ => {}
                }

                if let Ok(currency_data) = final_data {
                    CommandResponseObject::interactive_with_feedback(
                        CreateComponents::default(),
                        format!("Successfully modified currency **{}** `{}`", currency_data.currency_name, currency_data.currency_code),
                        format!("{0} modified currency {1}:\n> {2}",
                                data.user,
                                match action.name.as_str() {
                                    "code" | "state" => format!("**{}**", currency_data.currency_name),
                                    _ => format!("`{}`", currency_data.currency_code)
                                },
                                match action.name.as_str() {
                                    "code" => format!("Currency Code -> `{}`", currency_data.currency_code),
                                    "state" => format!("Nation/State -> *{}*", currency_data.state),
                                    "name" => format!("Currency Name -> **{}**", currency_data.currency_name),
                                    _ => "".into()
                                }
                            ),
                        false
                    )
                } else {
                    CommandResponseObject::text(format!("Couldn't respond to subcommand `modify`: Invalid data"))
                }
            },
            "recreate-database" => {
                CommandResponseObject::interactive(CreateComponents::default()
                    .create_action_row(|action_row| {
                        action_row
                            .create_button(|button| {
                                button
                                    .label("Confirm")
                                    .style(ButtonStyle::Danger)
                                    .custom_id("recreate-database-confirm")
                            })
                    .create_button(|button| {
                                button
                                    .label("Cancel")
                                    .style(ButtonStyle::Primary)
                                    .custom_id("recreate-database-cancel")
                            })
                    }).clone(),
                    "***DANGER***\nThis command ***CANNOT BE REVERSED***\nIf you click confirm, you will lose access to:\n- **All currencies, their metadata, circulation amount and reserves**\n- **All transaction history, for all currencies, forever**\n- **All current and past currency values, against the Gold Standard and each other**\nOnly use this command if you have been told to by the creator, `@Starkiller645`\nAre you _100% sure_ you want to continue?",
                    true
                    )
            },
            "list" => {
                let mut sort_by = CurrencySort::Name;
                let mut number = 10;

                for option in subcommand_data.options.clone() {
                    info!("{}", format!("{:?}", option));
                    match option.name.as_str() {
                        "sort" => {
                            if let Some(CommandDataOptionValue::String(sort)) = option.resolved {
								info!("Sorting by '{}'", sort.as_str());
                                sort_by = match sort.as_str() {
                                    "code" => CurrencySort::CurrencyCode,
                                    "name" => CurrencySort::Name,
                                    "state" => CurrencySort::State,
                                    "reserves" => CurrencySort::Reserves,
                                    "circulation" => CurrencySort::Circulation,
                                    "value" => CurrencySort::Value,
                                    _ => CurrencySort::CurrencyCode
                                };
                            } else {
                                tracing::error!("Couldn't get sort type")
                            }
                        },
                        "number" => if let Some(CommandDataOptionValue::Integer(num)) = option.resolved {
                            if num <= 0 {
                                return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: can't have a negative number of currencies to return."), true)
                            } else {
                                number = num
                            }
                        },
                        _ => {}
                    }
                };

                let result = self.query_agent.list_currencies(number, sort_by).await;

                let currencies = match result {
                    Ok(res) => res,
                    Err(e) => return CommandResponseObject::interactive(CreateComponents::default(), format!("{e:#?}"), true)
                };

                let mut currency_desc = "Currency Name";
                let mut code_desc = "Code";
                let mut state_desc = "Nation/State";
                let mut reserve_desc = "Gold Reserves";
                let mut circulation_desc = "Circulation";
                let mut value_desc = "Value";

                match sort_by {
                    CurrencySort::Name => currency_desc = "\u{001b}[1;32mCurrency Name\u{001b}[0m",
                    CurrencySort::CurrencyCode => code_desc = "\u{001b}[1;32mCode\u{001b}[0m",
                    CurrencySort::State => state_desc = "\u{001b}[1;32mNation/State\u{001b}[0m",
                    CurrencySort::Reserves => reserve_desc = "\u{001b}[1;32mGold Reserves\u{001b}[0m",
                    CurrencySort::Circulation => circulation_desc = "\u{001b}[1;32mCirculation\u{001b}[0m",
                    CurrencySort::Value => value_desc = "\u{001b}[1;32mValue\u{001b}[0m"
                };

                let mut list = "**Currency List**\n```ansi\n┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━┳━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━┓".to_string();
                list += format!("\n┃{code_desc} and {currency_desc}              ┃{state_desc}                  ┃{reserve_desc} ┃{circulation_desc}┃{value_desc}            ┃").as_str();
                list += "\n┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━╋━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━┫";

                // Currency name: 40 chars
                // Currency code: 5 chars (including [])
                // Currency state: 40 chars
                // Reserves: 14 chars
                // Circulation: 14 chars
                // Value: 19 chars
                // Lines: 7 chars
                // Total: 139 chars

                for currency in currencies {
                    list += format!(
                        "\n┃[\u{001b}[36m{0: <3.3}\u{001b}[0m] \u{001b}[1m{1: <30.30}\u{001b}[0m┃{5: <30.30}┃\u{001b}[1;33m{2: >7.7}\u{001b}[0m ingots┃\u{001b}[1;34m{3: >7.7}\u{001b}[0m {0}┃\u{001b}[1;35m{4: <3.3}\u{001b}[0m {0} / ingot┃",
                        currency.currency_code,
                        currency.currency_name,
                        currency.reserves,
                        currency.circulation,
                        currency.value,
                        currency.state
                    ).as_str()
                }

                list += "\n┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━┻━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━┛```";

                CommandResponseObject::interactive(CreateComponents::default(), list, true)
            },
            "view" => {
                let mut currency_code = None;
                for option in subcommand_data.options.clone() {
                    match option.name.as_str() {
                        "code" => currency_code = if let Some(CommandDataOptionValue::String(code)) = option.resolved {
                            Some(code)
                        } else {
                            None
                        },
                        _ => {}
                    }
                };

                match currency_code {
                    None => CommandResponseObject::interactive(CreateComponents::default(), format!("Couldn't respond to subcommand `view`: No currency code specified!"), true),
                    Some(code) => match self.query_agent.get_currency_data(code).await {
                        Ok(data) => CommandResponseObject::interactive(
                            CreateComponents::default(),
                            format!(
                                "`{0}` **{1}**\n> Nation/State: _{2}_\n> Reserves: `{3} ingots`\n> Circulation: `{4} {0}`\n> Value: `{5:.3} {0} / ingot`",
                                data.currency_code,
                                data.currency_name,
                                data.state,
                                data.reserves,
                                data.circulation,
                                data.value
                            ),
                            true
                        ),
                        Err(e) => CommandResponseObject::interactive(CreateComponents::default(), format!("Error looking up currency: {:?}", e), false)
                    }
                }
            }
            other => CommandResponseObject::text(format!("Couldn't respond to subcommand `{}`", other))
        }
    }

    pub async fn handle_component(&self, data: &MessageComponentInteraction, custom_data: &std::sync::Mutex<std::collections::HashMap<String, String>>) -> CommandResponseObject {
        info!("{data:?}");
        match data.data.custom_id.as_str() {
            "button-delete-confirm" => {
                let currency_target;
                let currency_name;
                {
                    let data = custom_data.lock().unwrap();
                    currency_target = match data.get("currency_code") {
                        Some(c) => c.clone(),
                        None => {
                            return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: this component interaction is invalid!"), true)
                        }
                    };
                    currency_name = match data.get("currency_name") {
                        Some(c) => c.clone(),
                        None => {
                            return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: this component interaction is invalid!"), true)
                        }
                    };
                }

                match self.manager.remove_currency(currency_target.clone())
                    .await {
                        Ok(_) => CommandResponseObject::interactive_with_feedback(CreateComponents::default(), format!("Successfully deleted currency **{currency_name}** `{currency_target}`"), format!("{} deleted currency **{}** `{}`", data.user, currency_name, currency_target), true),
                        Err(e) => CommandResponseObject::interactive(CreateComponents::default(), format!("Error: SQLx error: \n{e:?}"), true)
                }
            }
            "button-delete-cancel" => {
                CommandResponseObject::interactive_with_feedback(CreateComponents::default(), "Will not delete currency.", "", true)
            },
            "gold-transaction-confirm" => {

                let transaction_initiator: String;
                let transaction_amount: i64;
                let transaction_code: String;

                {
                    let data = custom_data.lock().unwrap();
                    transaction_initiator = match data.get("transaction_initiator") {
                        Some(s) => s.clone(),
                        None => {
                            return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: this component interaction is invalid!"), true)
                        }
                    };

                    transaction_amount = match data.get("transaction_amount") {
                        Some(s) => match s.parse() {
                            Ok(i) => i,
                            Err(_e) => {
                                return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: this component interaction is invalid!"), true)
                            }
                        },
                        None => {
                            return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: this component interaction is invalid!"), true)
                        }
                    };

                    transaction_code = match data.get("transaction_code") {
                        Some(s) => s.clone(),
                        None => {
                            return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: this component interaction is invalid!"), true)
                        }
                    }
                }

                let transaction_response = match self.manager.reserve_modify(transaction_code.clone(), transaction_amount, transaction_initiator).await {
                    Ok(data) => data,
                    Err(e) => return CommandResponseObject::interactive_with_feedback(CreateComponents::default(), format!("Error while completing transaction: `{e:?}`"), "", true)
                };

                let currency_data = match self.query_agent.get_currency_data(transaction_code.clone()).await {
                    Ok(data) => data,
                    Err(e) => return CommandResponseObject::interactive_with_feedback(CreateComponents::default(), format!("Error while completing currency balance check: `{e:?}`"), "", true)
                };

                let feedback = format!("Successfully completed gold reserve transaction!");
                let broadcast = format!("{0} made a gold reserve transaction:\n> Currency: **{1}** `{2}`\n> Nation/State: *{6}*\n> Amount: `{3} ingots`\n> New balance: `{4} ingots`\n> Transaction ID: `#{5:0>5}`", data.user, currency_data.currency_name, transaction_code, transaction_amount, currency_data.reserves, transaction_response.transaction_id, currency_data.state);

                CommandResponseObject::interactive_with_feedback(CreateComponents::default(), feedback, broadcast, true)
            }
            "currency-transaction-confirm" => {
                let transaction_initiator: String;
                let transaction_amount: i64;
                let transaction_code: String;

                {
                    let data = custom_data.lock().unwrap();
                    transaction_initiator = match data.get("transaction_initiator") {
                        Some(s) => s.clone(),
                        None => {
                            return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: this component interaction is invalid!"), true)
                        }
                    };

                    transaction_amount = match data.get("transaction_amount") {
                        Some(s) => match s.parse() {
                            Ok(i) => i,
                            Err(_e) => {
                                return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: this component interaction is invalid!"), true)
                            }
                        },
                        None => {
                            return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: this component interaction is invalid!"), true)
                        }
                    };

                    transaction_code = match data.get("transaction_code") {
                        Some(s) => s.clone(),
                        None => {
                            return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: this component interaction is invalid!"), true)
                        }
                    }
                }

                let transaction_response = match self.manager.circulation_modify(transaction_code.clone(), transaction_amount, transaction_initiator).await {
                    Ok(data) => data,
                    Err(e) => return CommandResponseObject::interactive_with_feedback(CreateComponents::default(), format!("Error while completing transaction: `{e:?}`"), "", true)
                };

                let currency_data = match self.query_agent.get_currency_data(transaction_code.clone()).await {
                    Ok(data) => data,
                    Err(e) => return CommandResponseObject::interactive_with_feedback(CreateComponents::default(), format!("Error while completing currency balance check: `{e:?}`"), "", true)
                };

                let feedback = format!("Successfully completed currency circulation transaction!");
                let broadcast = format!("{0} made a currency circulation transaction:\n> Currency: **{1}** `{2}`\n> Nation/State: *{6}*\n> Amount: `{3}{2}`\n> New balance: `{4}{2}`\n> Transaction ID: `#{5:0>5}`", data.user, currency_data.currency_name, transaction_code, transaction_amount, currency_data.circulation, transaction_response.transaction_id, currency_data.state);

                CommandResponseObject::interactive_with_feedback(CreateComponents::default(), feedback, broadcast, true)
            }
            "recreate-database-confirm" => {
                match self.manager.danger_recreate_database().await {
                    Ok(_) => CommandResponseObject::interactive_with_feedback(CreateComponents::default(), "Well, it's done. Hopefully you meant to do that, otherwise you're going to be in *big* trouble", format!("{0} recreated the entire database. *Any and all* stored data has been lost.", data.user), true),
                    Err(e) => CommandResponseObject::interactive_with_feedback(CreateComponents::default(), format!("We encountered an error recreating the database: {e:?}.\n*This is probably a good thing...*"), "", true)
                }
            },
            "recreate-database-cancel" => {
                CommandResponseObject::interactive_with_feedback(CreateComponents::default(), "Cancelled deleting database. This is probably a good thing.", "", true)
            }
            _ => CommandResponseObject::text("Unknown component, uh oh!"),
        }
    }
}
