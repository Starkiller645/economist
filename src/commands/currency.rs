use crate::{get_sql_connection, CommandResponseObject};
use serenity::builder::{
    CreateApplicationCommand, CreateApplicationCommandOption, CreateComponents,
};
use serenity::model::application::component::ButtonStyle;
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
};

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
}

pub async fn run(data: &ApplicationCommandInteraction, custom_data: &std::sync::Mutex<std::collections::HashMap<String, String>>) -> CommandResponseObject {
    let currency_data = data.data
        .options.get(0).unwrap()
        .options.clone();
    
    let mut currency_name = String::new();
    let mut currency_code = String::new();
    let mut circulation: i64 = 0;
    let mut gold_reserve: i64 = 0;

    for option in currency_data {
        match option.name.as_str() {
            "code" => { if let CommandDataOptionValue::String(code) = option.resolved.unwrap() {
                currency_code = code;
            }},
            "name" => { if let CommandDataOptionValue::String(name) = option.resolved.unwrap() {
                currency_name = name;
            }},
            "initial_circulation" => { if let CommandDataOptionValue::Integer(initial_circulation) = option.resolved.unwrap() {
                circulation = initial_circulation;
            }},
            "initial_reserve" => { if let CommandDataOptionValue::Integer(initial_reserve) = option.resolved.unwrap() {
                gold_reserve = initial_reserve;
            }},
            _ => {}
        }
    }

    match data.data.options.get(0).unwrap().name.as_str() {
        "remove" => {
            let mut currency_name = String::new();
            {
                let mut custom_data = custom_data.lock().unwrap();
                custom_data.insert("currency_id".into(), currency_code.clone());
            }

            match sqlx::query!("SELECT currency_name FROM currencies WHERE currency_id = ?", currency_code.clone()).fetch_one(&mut get_sql_connection().await.unwrap()).await {
                
                Ok(data) => {
                    currency_name = data.currency_name;
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

            let mut sql_conn = get_sql_connection().await.unwrap();
            match sqlx::query("INSERT INTO currencies(currency_id, currency_name, in_circulation, gold_reserve) VALUES (?, ?, ?, ?)")
                .bind(currency_code.clone())
                .bind(currency_name.clone())
                .bind(circulation)
                .bind(gold_reserve)
                .execute(&mut sql_conn).await {
                    Ok(_conn) => {
                        {
                            let mut custom_data = custom_data.lock().unwrap();
                            custom_data.insert("currency_id".into(), currency_code.clone());
                            custom_data.insert("currency_name".into(), currency_name.clone());
                            println!("Changed data: {:#?}", custom_data);
                        }
                        CommandResponseObject::text(
                            format!(
                                    "Created new currency **{0}**\nCurrency Code: *{1}*\nInitial circulation: {2}{1}\nInitial gold reserve: {3} ingots",
                                    currency_name,
                                    currency_code,
                                    circulation,
                                    gold_reserve
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
        other => CommandResponseObject::text(format!("Couldn't respond to subcommand `{}`", other))
    }
}

pub async fn handle_component(data: &MessageComponentInteraction, custom_data: &std::sync::Mutex<std::collections::HashMap<String, String>>) -> CommandResponseObject {
    match data.data.custom_id.as_str() {
        "button-delete-confirm" => {
            let currency_target;
            let mut currency_name = String::new();
            {
                let data = custom_data.lock().unwrap();
                currency_target = data.get("currency_id").unwrap().clone();
                currency_name = data.get("currency_name").unwrap().clone();
            }
            let mut sql_con = get_sql_connection().await.unwrap();
            sqlx::query("DELETE FROM currencies WHERE currency_id = ?;")
                .bind(currency_target.clone())
                .execute(&mut sql_con)
                .await.unwrap();
            CommandResponseObject::text(format!("Confirmed, deleted currency {} ({})", currency_target, currency_name))
        }
        "button-delete-cancel" => CommandResponseObject::text("Not deleting after all :D"),
        _ => CommandResponseObject::text("Unknown component, uh oh!"),
    }
}
