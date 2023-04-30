use crate::{get_sql_connection, CommandResponseObject};
use crate::commands::manage::*;
use serenity::builder::{
    CreateApplicationCommand, CreateApplicationCommandOption, CreateComponents,
};
use serenity::model::application::component::ButtonStyle;
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
};
use sqlx::Row;
use chrono::DateTime;
use chrono::offset::Utc;

#[derive(sqlx::FromRow)]
pub struct CurrencyData {
    pub currency_id: u64,
    pub currency_name: String,
    pub currency_code: String,
    pub circulation: i64,
    pub reserves: i64,
    pub state: String
}

#[derive(sqlx::FromRow)]
pub struct TransactionData {
    pub transaction_id: u64,
    pub transaction_date: DateTime<Utc>,
    pub currency_code: String,
    pub delta_reserves: Option<i64>,
    pub delta_circulation: Option<i64>
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("currency")
        .description("Economist: Manage currencies and their circulation levels (testing)")
        .create_option(|option| {
            option
                .kind(CommandOptionType::SubCommand)
                .name("add")
                .description("Add: add a new currency to the database")
                .create_sub_option(|option| {
                    option
                        .kind(CommandOptionType::String)
                        .name("code")
                        .description("Code: a three-letter currency code. Will throw an error if it's already in use.")
                        .min_length(3)
                        .max_length(3)
                        .required(true)
                })
                .create_sub_option(|option| {
                    option
                        .kind(CommandOptionType::String)
                        .name("name")
                        .description("Name: the name of your new currency! Note this does *not* need to be unique.")
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
                .name("remove")
                .description("Remove: remove a currency from the database")
                .create_sub_option(|option| {
                    option
                        .kind(CommandOptionType::String)
                        .name("code")
                        .description("Code: the three-letter currency code to remove.")
                        .min_length(3)
                        .max_length(3)
                        .required(true)
                })
        })
        .create_option(|option| {
            option
                .kind(CommandOptionType::SubCommandGroup)
                .name("reserve")
                .description("Reserve: manage gold reserves of a currency")
                .create_sub_option(|option| {
                    option
                        .kind(CommandOptionType::SubCommand)
                        .name("add")
                        .description("Add: add gold to a currency's reserves")
                        .create_sub_option(|option| {
                            option
                                .kind(CommandOptionType::Number)
                                .name("amount")
                                .description("Amount: the amount of gold to add to the reserves")
                                .required(true)
                        })
                        .create_sub_option(|option| {
                            option
                                .kind(CommandOptionType::String) 
                                .name("code")
                                .description("Code: the three-letter code of the target currency")
                                .required(true)
                        })
                })
        })
}

pub async fn run(data: &ApplicationCommandInteraction, custom_data: &std::sync::Mutex<std::collections::HashMap<String, String>>) -> CommandResponseObject {

    let mut sql_conn = match get_sql_connection().await {
        Ok(conn) => conn,
        Err(e) => return CommandResponseObject::interactive(
            CreateComponents::default(),
            format!("Error: Economist Bot encountered an error connecting to the MySQL database:\n{e:?}")
        )
    };

    let subcommand_data = match data.data
        .options.get(0) {
            Some(data) => data,
            None => return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: Economist Bot encountered an error processing a command. A debug dump of the command interaction is below:\n{data:?}"))
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
        "remove" => {
            let mut currency_name = String::new();
            {
                let mut custom_data = custom_data.lock().unwrap();
                custom_data.insert("currency_code".into(), currency_code.clone());
            }

            match sqlx::query("SELECT currency_name FROM currencies WHERE currency_code = ?")
                .bind(currency_code.clone())
                .fetch_one(&mut sql_conn).await {
                
                Ok(data) => {
                    currency_name = data.try_get("currency_name").unwrap();
                },
                Err(e) => {
                    return CommandResponseObject::text(format!("Error deleting currency: could not find the currency code `{currency_code}`"))
                }
            }
            {
                let mut custom_data = custom_data.lock().unwrap();
                custom_data.insert("currency_name".into(), currency_name.clone());
                println!("Changed data: {:#?}", custom_data);
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
                                    .style(ButtonStyle::Secondary)
                            })
                        /*action_row.create_input_text(|input| {
                            input
                                .label("Hello")
                                .custom_id("hello-input")
                                .style(InputTextStyle::Short)
                        })*/
                    })
                    .clone(),
            format!("Confirm you really want to delete the currency **{currency_name}**?\n*This is not reversible*")
            )
        }

        "add" => {
            match add_currency(currency_code.clone(), currency_name.clone(), circulation, gold_reserve, state).await
            {
                Ok(data) => {
                    {
                        let mut custom_data = custom_data.lock().unwrap();
                        custom_data.insert("currency_code".into(), currency_code.clone());
                        custom_data.insert("currency_name".into(), currency_name.clone());
                        println!("Changed data: {:#?}", custom_data);
                    }
                    CommandResponseObject::text(
                        format!(
                                "Created new currency **{0}** *({4})*\nCurrency Code: **{1}**\nInitial circulation: {2}{1}\nInitial gold reserve: {3} ingots",
                                data.currency_name,
                                data.currency_code,
                                data.circulation,
                                data.reserves,
                                data.state
                        )
                    )
                },
                Err(e) => {
                    let error_message = format!("{:?}", e);
                    if let Some(error) = e.into_database_error() {
                        match error.downcast_ref::<sqlx::mysql::MySqlDatabaseError>().number() {
                            1062 => CommandResponseObject::text(
                                format!(
                                    "Error creating currency **{currency_name}**:\nThe currency code `{currency_code}` is already in use, please choose another one!"
                                )
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
                None => return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: Economist Bot encountered an error processing a command. A debug dump of the command interaction is below:\n{data:?}"))
            };
            match action.name.as_str() {
                "add" => {
                    let mut currency_code = String::new();
                    let mut amount: i64 = 0;

                    for option in action.options.clone() {
                        match option.name.as_str() {
                            "code" => { if let Some(CommandDataOptionValue::String(code)) = option.resolved {
                                currency_code = code;
                            }},
                            "amount" => { if let Some(CommandDataOptionValue::Integer(transaction_amount)) = option.resolved {
                                amount = transaction_amount
                            }},
                            _ => {}
                        }
                    }

                    {
                        let mut data = custom_data.lock().unwrap();
                        data.insert("transaction_code".into(), currency_code.clone());
                        data.insert("transaction_amount".into(), format!("{}", amount));
                    }

                    match get_currency_data(currency_code.clone()).await {
                        Ok(data) => {
                            CommandResponseObject::interactive(
                                CreateComponents::default()
                                    .create_action_row(|action_row| {
                                        action_row
                                            .create_button(|button| {
                                                button
                                                    .label("Confirm")
                                                    .style(ButtonStyle::Primary)
                                                    .custom_id("transaction-confirm")
                                            })
                                            .create_button(|button| {
                                                button
                                                    .label("Cancel")
                                                    .style(ButtonStyle::Danger)
                                                    .custom_id("transaction-cancel")
                                            })
                                    }).clone(),
                                format!("**Review gold reserve transaction**\nCurrency: `{}` (*{}*)\nNation/State: *{}*\nGold reserve change: `{amount}`\nReserves after transaction: `{}`", data.currency_code, data.currency_name, data.state, (data.reserves + amount))
                            )
                        },
                        Err(e) => {
                            return match e {
                                sqlx::Error::RowNotFound => CommandResponseObject::interactive(CreateComponents::default(), format!("Error: the currency code `{}` was not found. Check your spelling and try again.", currency_code)),
                                _ => CommandResponseObject::interactive(CreateComponents::default(), format!("Error: SQLx Error\n`{:?}`", e))
                            }
                        }
                    }
                },
                "remove" => {
                    CommandResponseObject::text("")
                }
                _ => CommandResponseObject::text("")
            }
        }
        other => CommandResponseObject::text(format!("Couldn't respond to subcommand `{}`", other))
    }
}

pub async fn handle_component(data: &MessageComponentInteraction, custom_data: &std::sync::Mutex<std::collections::HashMap<String, String>>) -> CommandResponseObject {
    let mut sql_conn = match get_sql_connection().await {
        Ok(conn) => conn,
        Err(e) => return CommandResponseObject::interactive(
            CreateComponents::default(),
            format!("Error: Economist Bot encountered an error connecting to the MySQL database:\n{e:?}")
        )
    };
    match data.data.custom_id.as_str() {
        "button-delete-confirm" => {
            let currency_target;
            let currency_name;
            {
                let data = custom_data.lock().unwrap();
                currency_target = match data.get("currency_code") {
                    Some(c) => c.clone(),
                    None => {
                        return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: this component interaction is invalid!"))
                    }
                };
                currency_name = match data.get("currency_name") {
                    Some(c) => c.clone(),
                    None => {
                        return CommandResponseObject::interactive(CreateComponents::default(), format!("Error: this component interaction is invalid!"))
                    }
                };
            }

            println!("{data:#?}");
            match sqlx::query("DELETE FROM currencies WHERE currency_code = ?;")
                .bind(currency_target.clone())
                .execute(&mut sql_conn)
                .await {
                    Ok(_) => CommandResponseObject::text(format!("{} deleted currency {} ({})", data.user, currency_target, currency_name)),
                    Err(e) => CommandResponseObject::interactive(CreateComponents::default(), "")
            }
        }
        "button-delete-cancel" => {
            CommandResponseObject::text("Not deleting after all :D")
        },
        _ => CommandResponseObject::text("Unknown component, uh oh!"),
    }
}
