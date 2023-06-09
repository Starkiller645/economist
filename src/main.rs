/*
    Economist Bot - a Discord bot for tracking virtual currencies
    Copyright (C) 2023  Tallie Tye

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use anyhow::anyhow;
use tracing::{error, info, debug, warn};
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
use serenity::model::application::command::Command;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use crate::commands::manage::DBManager;
use crate::commands::query::DBQueryAgent;
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

    let password = match secret_store.get("DATABASE_PASSWORD") {
        Some(p) => p,
        None => {
            warn!("WARNING: you have not set a database password in you Secrets.toml");
            warn!("         This will allow ANYONE WITH PERMISSIONS to manage Economist Bot's database");
            warn!("         You should change this ASAP.");
            String::from("")
        }
    };

    info!("Starting workers...");
    let pool_clone = pool.clone();
    let (_tx, rx) = futures::channel::mpsc::channel(8);
    task::spawn(record_worker(persist_instance, pool_clone, rx));

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::DIRECT_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let circulation_handler = Arc::new(Mutex::new(circulation::CirculationHandler::new()));
    let reserve_handler = Arc::new(Mutex::new(reserve::ReserveHandler::new()));
    let list_handler = Arc::new(Mutex::new(list::ListHandler::new()));
    let view_handler = Arc::new(Mutex::new(view::ViewHandler::new()));
    let create_handler = Arc::new(Mutex::new(create::CreateHandler::new()));
    let delete_handler = Arc::new(Mutex::new(delete::DeleteHandler::new()));
    let modify_handler = Arc::new(Mutex::new(modify::ModifyHandler::new()));
    let records_handler = Arc::new(Mutex::new(records::RecordsHandler::new()));
    let database_handler = Arc::new(Mutex::new(database::DatabaseHandler::new(password)));

    let cmd_handlers: Vec<Arc<Mutex<dyn ApplicationCommandHandler + Send + Sync>>> = vec![
        circulation_handler.clone(),
        reserve_handler.clone(),
        delete_handler.clone(),
        database_handler.clone(),
        list_handler,
        view_handler,
        create_handler,
        modify_handler,
        records_handler,
    ];
    let interaction_handlers: Vec<Arc<Mutex<dyn InteractionResponseHandler + Send + Sync>>> = vec![
        circulation_handler,
        reserve_handler,
        delete_handler,
    ];

    let modal_handlers: Vec<Arc<Mutex<dyn ModalSubmitHandler + Send + Sync>>> = vec![
        database_handler
    ];

    let client = match Client::builder(&discord_token, intents).event_handler(Handler::new(secret_store, pool, cmd_handlers, interaction_handlers, modal_handlers)).await{
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
    ephemeral: bool,
    modal: bool,
}


impl CommandResponseObject {
    pub fn interactive(data: serenity::builder::CreateComponents, prompt: impl Into<String>, ephemeral: bool) -> Self {
        CommandResponseObject {
            interactive: true,
            interactive_data: Some(data),
            data: Some(prompt.into()),
            feedback: None,
            embed: None,
            ephemeral,
            modal: false
        }
    }

    pub fn interactive_only(data: serenity::builder::CreateComponents, ephemeral: bool) -> Self {
        CommandResponseObject {
            interactive: true,
            interactive_data: Some(data),
            data: None,
            feedback: None,
            embed: None,
            ephemeral,
            modal: false
        }
    }

    pub fn modal(data: serenity::builder::CreateComponents, custom_id: String) -> Self {
        CommandResponseObject {
            interactive: true,
            interactive_data: Some(data),
            data: Some(custom_id),
            feedback: None,
            embed: None,
            ephemeral: true,
            modal: true
        }
    }

    pub fn interactive_with_feedback(data: serenity::builder::CreateComponents, feedback: impl Into<String>, display: impl Into<String>, ephemeral: bool) -> Self {
        CommandResponseObject {
            interactive: true,
            interactive_data: Some(data),
            data: Some(display.into()),
            feedback: Some(feedback.into()),
            embed: None,
            ephemeral,
            modal: false
        }
    }

    pub fn text(data: impl Into<String>) -> Self {
        CommandResponseObject {
            interactive: false,
            interactive_data: None,
            data: Some(data.into()),
            feedback: None,
            embed: None,
            ephemeral: false,
            modal: false
        }
    }

    pub fn embed(data: serenity::builder::CreateEmbed) -> Self {
        CommandResponseObject {
            interactive: false,
            interactive_data: None,
            data: None,
            feedback: None,
            embed: Some(data),
            ephemeral: false,
            modal: false
        }
    }
    
    pub fn error(data: impl Display) -> Self {
        CommandResponseObject {
            interactive: false,
            interactive_data: None,
            data: Some(format!("Economist Bot encountered an error processing a command: \n`{data}`")),
            feedback: None,
            embed: None,
            ephemeral: true,
            modal: false
        }
    }

    pub fn is_interactive(&self) -> bool {
        self.interactive
    }

    pub fn is_ephemeral(&self) -> bool {
        self.ephemeral
    }

    pub fn is_modal(&self) -> bool {
        self.modal
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
    db_manager: DBManager,
    query_agent: DBQueryAgent,
    application_command_handlers: Vec<Arc<Mutex<dyn ApplicationCommandHandler + Send + Sync>>>,
    interaction_response_handlers: Vec<Arc<Mutex<dyn InteractionResponseHandler + Send + Sync>>>,
    modal_submit_handlers: Vec<Arc<Mutex<dyn ModalSubmitHandler + Send + Sync>>>
}

impl Handler {
    fn new(_secrets: SecretStore, pool: sqlx::postgres::PgPool, cmd_handlers: Vec<Arc<Mutex<dyn ApplicationCommandHandler + Send + Sync>>>, interaction_handlers: Vec<Arc<Mutex<dyn InteractionResponseHandler + Send + Sync>>>, modal_handlers: Vec<Arc<Mutex<dyn ModalSubmitHandler + Send + Sync>>>) -> Self {
        let db_manager = DBManager::new(pool.clone());
        let query_agent = DBQueryAgent::new(pool);
        Handler {
            db_manager,
            query_agent,
            application_command_handlers: cmd_handlers,
            interaction_response_handlers: interaction_handlers,
            modal_submit_handlers: modal_handlers
        }
    }
}

#[async_trait]
impl<'a> EventHandler for Handler {
    async fn interaction_create(&self, cx: Context, interaction: Interaction) {
        
        if let Interaction::ApplicationCommand(cmd) = interaction {
            let mut content = CommandResponseObject::text("Content unavailable: no response handler registered for command!");


            for handler in &self.application_command_handlers {
                let lock = handler.lock().await;
                let name: String = lock.get_name().into();
                drop(lock);

                if cmd.data.name.as_str() == "economist" {
                    content = commands::meta::run(&cmd)
                } else {
                    if let Some(sub_command) = cmd.data.options.get(0) {
                        if sub_command.name.as_str() == name {
                            let mut lock = handler.lock().await;
                            content = match lock.handle_application_command(&cmd, &self.query_agent, &self.db_manager).await {
                                Ok(data) => data.clone(),
                                Err(e) => CommandResponseObject::error(format!("Error responding to application command: {e:?}"))
                            };
                        }
                    }
                }
            }

            if content.is_modal() {
                if let Err(e) = cmd
                    .create_interaction_response(&cx.http, |response| {
                        response
                            .kind(InteractionResponseType::Modal)
                            .interaction_response_data(|message| {
                                message
                                    .set_components(content.get_interactive_data().clone())
                                    .custom_id(content.get_text().clone())
                                    .title(cmd.data.name.clone())
                            })
                    }).await {
                        error!("Cannot create modal response to slash command: {e:?}")
                    }
            } else if let Some(embed) = content.embed.clone() {
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
                        if let Err(e) = cmd
                            .create_interaction_response(&cx.http, |response| {
                                response
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| 
                                                               message
                                                               .content(content.get_text())
                                                               .ephemeral(content.is_ephemeral())
                                    )
                            })
                            .await
                        {
                            debug!("Cannot respond to slash command: {}", e);
                            debug!("Debug dump: {:?}", content.get_text())
                        }
                    }
                }
            }
        } else if let Interaction::MessageComponent(cmd) = interaction {
            let mut content = CommandResponseObject::error("Got no response from interaction response handler");
            for interaction_response in &self.interaction_response_handlers {
                let interaction_pattern;
                let guard = interaction_response.lock().await;
                interaction_pattern = guard.get_pattern();
                for interaction_callsign in interaction_pattern.clone() {
                    if interaction_callsign == cmd.data.custom_id.as_str() {
                        content = match guard.handle_interaction_response(&cmd, &self.query_agent, &self.db_manager).await {
                            Ok(data) => data,
                            Err(e) => CommandResponseObject::error(format!("{e:?}"))
                        }
                    }
                }
            }

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
                    /*if let Err(e) = cmd.channel_id.say(&cx.http, content.get_text()).await {
                        debug!("Could not post global message: {e:?}")
                    }*/
                }
                false => {
                    if let Err(e) = cmd
                        .create_interaction_response(&cx.http, |response| {
                            response
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|message| message
                                                           .content(content.get_text())
                                                           .ephemeral(content.is_ephemeral()))
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
        } else if let Interaction::ModalSubmit(cmd) = interaction {
            let mut content = CommandResponseObject::error("Got no response from interaction response handler");
            info!("Command data: {cmd:#?}");
            info!("Matching on callsign: {}", cmd.data.custom_id.clone());
            for interaction_response in &self.modal_submit_handlers {
                let interaction_pattern;
                let guard = interaction_response.lock().await;
                interaction_pattern = guard.get_pattern();
                for interaction_callsign in interaction_pattern.clone() {
                    info!("Checking callsign {interaction_callsign}");
                    if interaction_callsign == cmd.data.custom_id.as_str() {
                        info!("Got a match!");
                        content = match guard.handle_modal_submit(&cmd, &self.query_agent, &self.db_manager).await {
                            Ok(data) => {
                                info!("Data: {data:#?}");
                                data
                            },
                            Err(e) => CommandResponseObject::error(format!("{e:?}"))
                        }
                    }
                }
            }
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
                                .interaction_response_data(|message| message
                                                           .content(content.get_text())
                                                           .ephemeral(content.is_ephemeral())
                                )
                        })
                        .await
                    {
                        debug!("Cannot respond to slash command: {}", e);
                    }
                    /*if let Err(e) = cmd.channel_id.say(&cx.http, content.get_text()).await {
                        debug!("Could not post global message: {e:?}")
                    }*/
                }
            }
        }
    }

    async fn ready(&self, cx: Context, ready: Ready) {
        info!("Bot `{}` is up and running!", ready.user.name);

        let mut sub_option_vec = vec![];
        for sub_option in &self.application_command_handlers {
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

        match Command::create_global_application_command(&cx.http, |command| {
            let mut cmd = command
                .name("currency")
                .description("Manage and view currencies and their circulation levels");
            for sub_option in sub_option_vec.clone() {
                cmd = cmd
                    .add_option(sub_option.clone())
            }
            cmd
        }).await {
            Ok(_) => {},
            Err(e) => error!("Error occurred setting application command `currency`: {e:?}")
        };
        match Command::create_global_application_command(&cx.http, |command| {
            commands::meta::register(command)
        }).await {
            Ok(_) => {},
            Err(e) => error!("Error occurred setting application command `economist`: {e:?}")
        };
    }
}

async fn sqlx_init(pool: &sqlx::postgres::PgPool) -> Result<(), sqlx::Error> {
    let postgres_version: String = sqlx::query("SELECT version()").fetch_one(pool).await?.try_get("version")?; 

    info!("PostgreSQL version: {}", postgres_version);

    sqlx::query("CREATE TABLE IF NOT EXISTS currencies(
        currency_id BIGSERIAL NOT NULL,
        currency_code TEXT NOT NULL UNIQUE,
        currency_name TEXT NOT NULL,
        state TEXT NOT NULL,
        circulation BIGINT NOT NULL,
        reserves BIGINT NOT NULL,
        owner TEXT NOT NULL,
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
    sqlx::any::AnyConnection::connect(url.as_str()).await
}
