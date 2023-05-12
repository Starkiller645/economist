use anyhow::anyhow;
use tracing::{error, info, debug};
use serenity::prelude::*;
use tokio::sync::Mutex;
use std::fs::create_dir_all;
use std::fmt::Display;
use std::sync::Arc;
use serenity::model::{
    gateway::Ready
};
use serenity::async_trait;
use serenity::builder::CreateApplicationCommandOption;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::id::GuildId;
use crate::commands::manage::DBManager;
use crate::commands::query::DBQueryAgent;
use crate::commands::currency::CurrencyHandler;
use crate::workers::records::*;
use sqlx::{Connection, Row};
use shuttle_secrets::SecretStore;
use shuttle_persist::PersistInstance;
use tokio::task;

pub mod commands;
pub mod workers;
pub mod types;
pub mod handlers;
pub mod utils;

use crate::types::*;
use crate::handlers::*;

pub mod consts {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[shuttle_runtime::main]
async fn serenity(
        #[shuttle_secrets::Secrets] secret_store: SecretStore,
        #[shuttle_shared_db::Postgres(local_uri = "{secrets.DATABASE_URL}")] pool: sqlx::postgres::PgPool,
        #[shuttle_persist::Persist] persist_instance: PersistInstance
    ) -> shuttle_serenity::ShuttleSerenity {
    info!("Loading Economist Bot...");
    
    //dotenvy::dotenv().expect("Error: Failed reading environment variables");
    
    create_dir_all("data/").unwrap();

    let Some(discord_token) = secret_store.get("DISCORD_TOKEN") else {
        return Err(anyhow!("Failed to get DISCORD_TOKEN from Shuttle secret store").into())
    };

    info!("Initialising SQL database...");

    let Some(_guild_id) = secret_store.get("DISCORD_GUILD_ID") else {
        return Err(anyhow!("Failed to get DISCORD_GUILD_ID from Shuttle secret store").into())
    };

    match sqlx_init(&pool).await {
        Ok(_) => {},
        Err(e) => {
            return Err(anyhow!("Error initialising SQL database: {e:?}").into());
        }
    };

    info!("Starting workers...");
    //let persistance = persist_instance.clone();
    let pool_clone = pool.clone();
    let (_tx, rx) = futures::channel::mpsc::channel(8);
    task::spawn(record_worker(persist_instance, pool_clone, rx));

    //let discord_token = env::var("DISCORD_TOKEN").expect("Error: DISCORD_TOKEN environment variable not set!");
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::DIRECT_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let circulation_handler = Arc::new(Mutex::new(circulation::CirculationHandler::new()));
    let reserve_handler = Arc::new(Mutex::new(reserve::ReserveHandler::new()));
    let list_handler = Arc::new(Mutex::new(list::ListHandler::new()));
    let view_handler = Arc::new(Mutex::new(view::ViewHandler::new()));
    let create_handler = Arc::new(Mutex::new(create::CreateHandler::new()));
    let delete_handler = Arc::new(Mutex::new(delete::DeleteHandler::new()));
    let modify_handler = Arc::new(Mutex::new(modify::ModifyHandler::new()));
    let records_handler = Arc::new(Mutex::new(records::RecordsHandler::new()));

    let cmd_handlers: Vec<Arc<Mutex<dyn ApplicationCommandHandler + Send + Sync>>> = vec![
        circulation_handler.clone(),
        reserve_handler.clone(),
        delete_handler.clone(),
        list_handler,
        view_handler,
        create_handler,
        modify_handler,
        records_handler
    ];
    let interaction_handlers: Vec<Arc<Mutex<dyn InteractionResponseHandler + Send + Sync>>> = vec![
        circulation_handler,
        reserve_handler,
        delete_handler
    ];

    let client = match Client::builder(&discord_token, intents).event_handler(Handler::new(secret_store, pool, cmd_handlers, interaction_handlers)).await{
        Ok(c) => c,
        Err(e) => return Err(anyhow!("Error creating client: {e:?}").into())
    };

    Ok(client.into())
}

#[derive(Clone, Debug)]
pub struct CommandResponseObject {
    interactive: bool,
    interactive_data: Option<serenity::builder::CreateComponents>,
    data: Option<String>,
    feedback: Option<String>,
    embed: Option<serenity::builder::CreateEmbed>,
    ephemeral: bool
}


impl CommandResponseObject {
    pub fn interactive(data: serenity::builder::CreateComponents, prompt: impl Into<String>, ephemeral: bool) -> Self {
        CommandResponseObject {
            interactive: true,
            interactive_data: Some(data),
            data: Some(prompt.into()),
            feedback: None,
            embed: None,
            ephemeral
        }
    }

    pub fn interactive_only(data: serenity::builder::CreateComponents, ephemeral: bool) -> Self {
        CommandResponseObject {
            interactive: true,
            interactive_data: Some(data),
            data: None,
            feedback: None,
            embed: None,
            ephemeral
        }
    }

    pub fn interactive_with_feedback(data: serenity::builder::CreateComponents, feedback: impl Into<String>, display: impl Into<String>, ephemeral: bool) -> Self {
        CommandResponseObject {
            interactive: true,
            interactive_data: Some(data),
            data: Some(display.into()),
            feedback: Some(feedback.into()),
            embed: None,
            ephemeral
        }
    }

    pub fn text(data: impl Into<String>) -> Self {
        CommandResponseObject {
            interactive: false,
            interactive_data: None,
            data: Some(data.into()),
            feedback: None,
            embed: None,
            ephemeral: false
        }
    }

    pub fn embed(data: serenity::builder::CreateEmbed) -> Self {
        CommandResponseObject {
            interactive: false,
            interactive_data: None,
            data: None,
            feedback: None,
            embed: Some(data),
            ephemeral: false
        }
    }
    
    pub fn error(data: impl Display) -> Self {
        CommandResponseObject {
            interactive: false,
            interactive_data: None,
            data: Some(format!("Economist Bot encountered an error processing a command: {data}")),
            feedback: None,
            embed: None,
            ephemeral: true
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
    secrets: SecretStore,
    currency_handler: CurrencyHandler
}

impl Handler {
    fn new(secrets: SecretStore, pool: sqlx::postgres::PgPool, cmd_handlers: Vec<Arc<Mutex<dyn ApplicationCommandHandler + Send + Sync>>>, interaction_handlers: Vec<Arc<Mutex<dyn InteractionResponseHandler + Send + Sync>>>) -> Self {
        let db_manager = DBManager::new(pool.clone());
        let query_agent = DBQueryAgent::new(pool);
        let currency_handler = CurrencyHandler::new(db_manager.clone(), query_agent, cmd_handlers, interaction_handlers);
        Handler {
            secrets,
            currency_handler
        }
    }
}

#[async_trait]
impl<'a> EventHandler for Handler {
    async fn interaction_create(&self, cx: Context, interaction: Interaction) {
        
        if let Interaction::ApplicationCommand(cmd) = interaction {
            let mut content = CommandResponseObject::text("Content unavailable: no response handler registered for command!");


            for handler in &self.currency_handler.application_command_handlers {
                let lock = handler.lock().await;
                let name: String = lock.get_name().into();
                drop(lock);

                if cmd.data.name.as_str() == "economist" {
                    content = commands::meta::run(&cmd)
                } else {
                    if let Some(sub_command) = cmd.data.options.get(0) {
                        if sub_command.name.as_str() == name {
                            let mut lock = handler.lock().await;
                            content = match lock.handle_application_command(&cmd, &self.currency_handler.query_agent, &self.currency_handler.manager).await {
                                Ok(data) => data.clone(),
                                Err(e) => CommandResponseObject::error(format!("Error responding to application command: {e:?}"))
                            };
                        }
                    }
                }
            }

            if let Some(embed) = content.embed.clone() {
                if let Err(e) = cmd
                    .create_interaction_response(&cx.http, |response| {
                        response
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| {
                                message
                                    .set_embed(embed)
                                    .title(cmd.data.name.clone())
                                    .ephemeral(content.is_ephemeral())
                            })
                    }).await {
                        error!("Cannot create embed response to slash command: {}", e);
                    }
                    
            } else {
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
                                error!("Cannot create interactive response to slash command: {}", e);
                                info!("Debug dump: {:#?}", content)
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
                                debug!("Cannot respond to slash command: {}", e);
                                debug!("Debug dump: {:?}", content.get_text())
                            }
                        //}
                    }
                }
            }
        } else if let Interaction::MessageComponent(cmd) = interaction {
            let mut content = CommandResponseObject::error("Got no response from interaction response handler");
            for interaction_response in &self.currency_handler.interaction_response_handlers {
                let interaction_pattern;
                let guard = interaction_response.lock().await;
                interaction_pattern = guard.get_pattern();
                for interaction_callsign in interaction_pattern.clone() {
                    if interaction_callsign == cmd.data.custom_id.as_str() {
                        content = match guard.handle_interaction_response(&cmd, &self.currency_handler.query_agent, &self.currency_handler.manager).await {
                            Ok(data) => data,
                            Err(e) => CommandResponseObject::error(format!("{e:?}"))
                        }
                    }
                }
            }

            /*let mut content = match cmd.data.custom_id.as_str() {
                "button-delete-confirm" | "button-delete-cancel" | "gold-transaction-confirm" | "gold-transaction-cancel" | "currency-transaction-confirm" | "currency-transaction-cancel" | "recreate-database-confirm" | "recreate-database-cancel" => self.currency_handler.handle_component(&cmd, &self.custom_data).await,
                _ => CommandResponseObject::text("Not handled :(")
            };*/
            match cmd.message.delete(&cx.http).await {
                 Ok(_) => {},
                 Err(e) => debug!("Error occurred deleting message: {e:?}")
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
                            debug!("Cannot respond to slash command: {}", e);
                            debug!("Debug dump: {:?}", content.get_interactive_data())
                        }
                    if let Err(e) = cmd.channel_id.say(&cx.http, content.get_text()).await {
                        debug!("Could not post global message: {e:?}")
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
                        debug!("Cannot respond to slash command: {}", e);
                    }
                    if let Err(e) = cmd.channel_id.say(&cx.http, content.get_text()).await {
                        debug!("Could not post global message: {e:?}")
                    }
                }
            }
        }
    }

    async fn ready(&self, cx: Context, ready: Ready) {
        info!("Bot `{}` is up and running!", ready.user.name);

        /*let command_currency = Command::create_global_application_command(&cx.http, |command| {
            commands::currency::register(command)
        })
        .await;

        let command_meta = Command::create_global_application_command(&cx.http, |command| {
            commands::meta::register(command)
        })
        .await;*/

        let guild_id = GuildId(self.secrets.get("DISCORD_GUILD_ID").unwrap().parse().unwrap());

        let mut sub_option_vec = vec![];
        for sub_option in &self.currency_handler.application_command_handlers {
            let sub_option_lock = sub_option.lock().await;

            let mut opt = CreateApplicationCommandOption::default();
            opt = opt
                .kind(sub_option_lock.get_option_kind())
                .name(sub_option_lock.get_name())
                .description(sub_option_lock.get_description()).clone();
            for op in sub_option_lock.register() {
                opt = opt
                    .add_sub_option(op)
                    .clone()
            }
            sub_option_vec.push(opt);
        }

        match guild_id.set_application_commands(&cx.http, |command| {
            command
                .create_application_command(|command| {
                    let mut cmd = command
                        .name("currency")
                        .description("Manage and view currencies and their circulation levels");
                    for sub_option in sub_option_vec {
                        cmd = cmd
                            .add_option(sub_option.clone())
                    }
                    cmd
                })
                .create_application_command(|command| commands::meta::register(command))
        }).await {
            Ok(_) => {},
            Err(e) => error!("Error occurred setting application commands: {e:?}")
        };
    }
}

async fn sqlx_init(pool: &sqlx::postgres::PgPool) -> Result<(), sqlx::Error> {
    /*let pool = MySqlPoolOptions::new()
        .max_connections(2)
        .connect(format!(
                "mysql://{0}:{1}@{2}/{3}",
                env::var("MYSQL_DATABASE_USER").unwrap(),
                env::var("MYSQL_DATABASE_PASSWORD").unwrap(),
                env::var("MYSQL_DATABASE_URL").unwrap(),
                env::var("MYSQL_DATABASE_NAME").unwrap()).as_str())
        .await?;*/

    let postgres_version: String = sqlx::query("SELECT version()").fetch_one(pool).await?.try_get("version")?; 

    info!("PostgreSQL version: {}", postgres_version);

    sqlx::query("CREATE TABLE IF NOT EXISTS currencies(
        currency_id BIGSERIAL NOT NULL,
        currency_code TEXT NOT NULL UNIQUE,
        currency_name TEXT NOT NULL,
        state TEXT NOT NULL,
        circulation BIGINT NOT NULL,
        reserves BIGINT NOT NULL,
        value DOUBLE PRECISION GENERATED ALWAYS AS (
            CASE WHEN reserves <= 0 THEN 0
                 WHEN circulation <= 0 THEN 0 
                 ELSE (
                     CAST(reserves AS DOUBLE PRECISION) / CAST(circulation AS DOUBLE PRECISION)
                 ) 
                 END
            ) STORED,
        PRIMARY KEY (currency_id)
    );").execute(pool).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS transactions(
        transaction_id BIGSERIAL NOT NULL,
        transaction_date TIMESTAMP WITHOUT TIME ZONE NOT NULL,
        currency_id BIGINT NOT NULL,
        delta_circulation BIGINT,
        delta_reserves BIGINT,
        initiator TEXT NOT NULL,
        PRIMARY KEY (transaction_id),
        FOREIGN KEY (currency_id) REFERENCES currencies(currency_id) ON DELETE CASCADE
    )").execute(pool).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS records(
        record_id BIGSERIAL NOT NULL,
        record_date DATE NOT NULL,
        currency_id BIGINT NOT NULL,
        opening_value DOUBLE PRECISION,
        closing_value DOUBLE PRECISION,
        delta_value DOUBLE PRECISION GENERATED ALWAYS AS (closing_value - opening_value) STORED,
        growth SMALLINT GENERATED ALWAYS AS (
            CASE WHEN (closing_value - opening_value) = 0 THEN 0
                 WHEN (closing_value - opening_value) > 0 THEN 1
                 ELSE -1
                 END
            ) STORED,
        PRIMARY KEY (record_id),
        FOREIGN KEY (currency_id) REFERENCES currencies(currency_id) ON DELETE CASCADE
        )
    ").execute(pool).await?;
    Ok(())
}

pub async fn get_sql_connection(url: String) -> Result<sqlx::any::AnyConnection, sqlx::Error> {
    /*sqlx::mysql::MySqlConnection::connect(format!(
            "mysql://{0}:{1}@{2}/{3}",
            env::var("MYSQL_DATABASE_USER").unwrap(),
            env::var("MYSQL_DATABASE_PASSWORD").unwrap(),
            env::var("MYSQL_DATABASE_URL").unwrap(),
            env::var("MYSQL_DATABASE_NAME").unwrap()).as_str())
    .await*/
    sqlx::any::AnyConnection::connect(url.as_str()).await
}

/*async fn generate_reports(database_url: String) {
    let mut conn = get_sql_connection(database_url).await.unwrap();

    let mut currency_codes = sqlx::query("SELECT currency_code, currency_id FROM currencies;")
        .fetch(&mut conn);

    while let Some(currency_code) = currency_codes.try_next().await.unwrap() {
        let code: String = currency_code.try_get("currency_code").unwrap();
        let id: i64 = currency_code.try_get("currency_id").unwrap();
    }
}*/
