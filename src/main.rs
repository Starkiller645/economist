use serenity::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;
use serenity::model::{
    channel::Message,
    gateway::Ready
};
use serenity::utils::MessageBuilder;
use serenity::async_trait;
use serenity::model::application::command::Command;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::id::GuildId;
use std::env;
use sqlx::mysql::{MySqlPoolOptions, MySqlPool, MySqlConnection};
use sqlx::Connection;
use sqlx::Row;
use lazy_static::lazy_static;
use futures::TryStreamExt;
use shuttle_secrets::SecretStore;

pub mod commands;

#[derive(Clone)]
pub struct CommandResponseObject {
    interactive: bool,
    interactive_data: Option<serenity::builder::CreateComponents>,
    data: Option<String>,
    feedback: Option<String>,
    ephemeral: bool
}

impl CommandResponseObject {
    pub fn interactive(data: serenity::builder::CreateComponents, prompt: impl Into<String>, ephemeral: bool) -> Self {
        CommandResponseObject {
            interactive: true,
            interactive_data: Some(data),
            data: Some(prompt.into()),
            feedback: None,
            ephemeral
        }
    }

    pub fn interactive_only(data: serenity::builder::CreateComponents, ephemeral: bool) -> Self {
        CommandResponseObject {
            interactive: true,
            interactive_data: Some(data),
            data: None,
            feedback: None,
            ephemeral
        }
    }

    pub fn interactive_with_feedback(data: serenity::builder::CreateComponents, feedback: impl Into<String>, display: impl Into<String>, ephemeral: bool) -> Self {
        CommandResponseObject {
            interactive: true,
            interactive_data: Some(data),
            data: Some(display.into()),
            feedback: Some(feedback.into()),
            ephemeral
        }
    }

    pub fn text(data: impl Into<String>) -> Self {
        CommandResponseObject {
            interactive: false,
            interactive_data: None,
            data: Some(data.into()),
            feedback: None,
            ephemeral: false
        }
    }

    pub fn is_interactive(&self) -> bool {
        self.interactive
    }

    pub fn is_ephemeral(&self) -> bool {
        self.ephemeral
    }

    pub fn get_text(&self) -> String {
        if let Some(text) = self.data.clone() {
            return text
        } else {
            return String::new()
        }
    }

    pub fn get_interactive_data(&mut self) -> &mut serenity::builder::CreateComponents {
        self.interactive_data.as_mut().unwrap()
    }
    
    pub fn get_feedback(&self) -> String {
        if let Some(text) = self.feedback.clone() {
            text
        } else {
            String::new()
        }
    }
}

struct Handler {
    custom_data: Mutex<HashMap<String, String>>
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, cx: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(e) = msg.channel_id.say(&cx.http, "pong!").await {
                eprintln!("Message: Error: {:?}", e);
            }
        }
    }

    async fn interaction_create(&self, cx: Context, interaction: Interaction) {
        
        if let Interaction::ApplicationCommand(cmd) = interaction {
            println!("Received command interaction: {:#?}", cmd);

            let mut content = match cmd.data.name.as_str() {
                "version" => commands::version::run(&cmd),
                "currency" => commands::currency::run(&cmd, &self.custom_data).await,
                _ => {
                    CommandResponseObject::text("Not implemented yet :(")
                }
            };

            println!("Custom data: {:#?}", self.custom_data);


            match content.is_interactive() {
                true => {
                    if let Err(e) = cmd
                        .create_interaction_response(&cx.http, |response| {
                            response
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|message| message
                                                           .set_components(content.get_interactive_data().clone())
                                                           .content(content.get_text().clone())
                                                           .ephemeral(content.is_ephemeral())
                                                           .custom_id(cmd.data.name.clone())
                                                           .title(cmd.data.name.clone()))
                        }).await {
                            println!("Cannot create interactive response to slash command: {}", e);
                            eprintln!("Debug dump: {:?}", content.get_interactive_data())
                        }
                }
                false => {
                    /*if content.data.as_str() == "##DELETE##" {
                        if let Err(e) = cmd
                            .create_interaction_response(&cx.http, |response|) {
                                response
                                    .kind(InteractionResponseType::)
                            }
                    } else {*/
                        if let Err(e) = cmd
                            .create_interaction_response(&cx.http, |response| {
                                response
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| message.content(content.get_text()))
                            })
                            .await
                        {
                            println!("Cannot respond to slash command: {}", e);
                            eprintln!("Debug dump: {:?}", content.get_text())
                        }
                    //}
                }
            }
        } else if let Interaction::MessageComponent(cmd) = interaction {
            let mut content = match cmd.data.custom_id.as_str() {
                "button-delete-confirm" | "button-delete-cancel" | "transaction-confirm" | "transaction-cancel" => commands::currency::handle_component(&cmd, &self.custom_data).await,
                _ => CommandResponseObject::text("Not handled :(")
            };
            match cmd.message.delete(&cx.http).await {
                 Ok(_) => {},
                 Err(e) => println!("Error occurred deleting message: {e:?}")
            };
            match content.is_interactive() {
                true => {
                    if let Err(e) = cmd
                        .create_interaction_response(&cx.http, |response| {
                            response
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|message| message
                                                           .set_components(content.get_interactive_data().clone())
                                                           .content(content.get_feedback().clone())
                                                           .ephemeral(content.is_ephemeral())
                                                           .custom_id(cmd.data.custom_id.clone())
                                                           .title(cmd.data.custom_id.clone()))
                        }).await {
                            println!("Cannot respond to slash command: {}", e);
                            eprintln!("Debug dump: {:?}", content.get_interactive_data())
                        }
                    if let Err(e) = cmd.channel_id.say(&cx.http, content.get_text()).await {
                        println!("Could not post global message: {e:?}")
                    }
                }
                false => {
                    if let Err(e) = cmd
                        .create_interaction_response(&cx.http, |response| {
                            response
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|message| message.content(content.get_text()))
                        })
                        .await
                    {
                        println!("Cannot respond to slash command: {}", e);
                    }
                    if let Err(e) = cmd.channel_id.say(&cx.http, content.get_text()).await {
                        println!("Could not post global message: {e:?}")
                    }
                }
            }
        }
        println!("Done handling!");
    }

    async fn ready(&self, cx: Context, ready: Ready) {
        println!("Bot `{}` is up and running!", ready.user.name);

        /*let command_currency = Command::create_global_application_command(&cx.http, |command| {
            commands::currency::register(command)
        })
        .await;

        println!("Created command `/currency`");

        let command_version = Command::create_global_application_command(&cx.http, |command| {
            commands::version::register(command)
        })
        .await;*/

        let guild_id = GuildId(env::var("GUILD_ID").unwrap().parse().unwrap());

        guild_id.set_application_commands(&cx.http, |commands| {
            commands
                .create_application_command(|command| commands::currency::register(command))
                .create_application_command(|command| commands::version::register(command))
        }).await.unwrap();

        println!("Created command `/version`");
    }
}

#[shuttle_runtime::main]
async fn serenity(
        #[shuttle_secrets::Secrets] secret_store: SecretStore
    ) -> shuttle_serenity::ShuttleSerenity {   
    dotenvy::dotenv().expect("Error: Failed reading environment variables");
    println!("Initialising SQL database...");
    sqlx_init().await.unwrap();
    println!("Done!");

    let discord_token = env::var("DISCORD_TOKEN").expect("Error: DISCORD_TOKEN environment variable not set!");
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::DIRECT_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&discord_token, intents).event_handler(Handler { custom_data: HashMap::new().into() }).await.expect("Error: could not create client");
    
    if let Err(e) = client.start().await {
        eprintln!("Client: Error: {:?}", e);
    }

    Ok(client.into())
}

async fn sqlx_init() -> Result<(), sqlx::Error> {
    let pool = MySqlPoolOptions::new()
        .max_connections(2)
        .connect(format!(
                "mysql://{0}:{1}@{2}/{3}",
                env::var("MYSQL_DATABASE_USER").unwrap(),
                env::var("MYSQL_DATABASE_PASSWORD").unwrap(),
                env::var("MYSQL_DATABASE_URL").unwrap(),
                env::var("MYSQL_DATABASE_NAME").unwrap()).as_str())
        .await?;

    sqlx::query("CREATE TABLE IF NOT EXISTS currencies(currency_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT, currency_code TEXT NOT NULL UNIQUE, currency_name TEXT NOT NULL, state TEXT NOT NULL, circulation BIGINT NOT NULL, reserves BIGINT NOT NULL, PRIMARY KEY (currency_id));").execute(&pool).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS transactions(transaction_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT, transaction_date DATE NOT NULL, currency_id BIGINT UNSIGNED NOT NULL, delta_circulation BIGINT, delta_reserves BIGINT, PRIMARY KEY (transaction_id), FOREIGN KEY (currency_id) REFERENCES currencies(currency_id) ON DELETE CASCADE)").execute(&pool).await?;
    Ok(())
}

pub async fn get_sql_connection() -> Result<sqlx::mysql::MySqlConnection, sqlx::Error> {
    sqlx::mysql::MySqlConnection::connect(format!(
            "mysql://{0}:{1}@{2}/{3}",
            env::var("MYSQL_DATABASE_USER").unwrap(),
            env::var("MYSQL_DATABASE_PASSWORD").unwrap(),
            env::var("MYSQL_DATABASE_URL").unwrap(),
            env::var("MYSQL_DATABASE_NAME").unwrap()).as_str())
    .await
}

async fn generate_reports() {
    let mut conn = get_sql_connection().await.unwrap();

    let mut currency_codes = sqlx::query("SELECT currency_code, currency_id FROM currencies;")
        .fetch(&mut conn);

    while let Some(currency_code) = currency_codes.try_next().await.unwrap() {
        let code: String = currency_code.try_get("currency_code").unwrap();
        let id: i64 = currency_code.try_get("currency_id").unwrap();
    }
}
