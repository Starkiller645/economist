use chrono::{offset::Utc, DateTime, NaiveDate};
use async_trait::async_trait;
use crate::CommandResponseObject;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::prelude::command::CommandOptionType;
use serenity::builder::CreateApplicationCommandOption;
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::model::application::interaction::modal::ModalSubmitInteraction;
use crate::commands::query::*;
use crate::commands::manage::*;

#[derive(sqlx::FromRow, Debug, Clone, Default)]
pub struct CurrencyData {
    pub currency_id: i64,
    pub currency_name: String,
    pub currency_code: String,
    pub circulation: i64,
    pub reserves: i64,
    pub owner: String,
    pub value: f64,
    pub state: String,
}

#[derive(sqlx::FromRow, Debug, Clone, Default)]
pub struct TransactionData {
    pub transaction_id: i64,
    pub transaction_date: DateTime<Utc>,
    pub currency_code: String,
    pub delta_reserves: Option<i64>,
    pub delta_circulation: Option<i64>,
}

#[derive(sqlx::FromRow, Debug, Clone, Default)]
pub struct RecordData {
    pub record_id: i64,
    pub record_date: NaiveDate,
    pub currency_id: i64,
    pub opening_value: f64,
    pub closing_value: f64,
    pub delta_value: f64,
    pub growth: i16, // -1 for decline, 0 for steady, 1 for growth
}

#[derive(Debug, Clone, Default)]
pub enum WorkerMessage {
    #[default]
    Halt,
}

#[async_trait]
pub trait ApplicationCommandHandler {
    async fn handle_application_command(&mut self, data: &ApplicationCommandInteraction, query_agent: &DBQueryAgent, manager: &DBManager) -> Result<CommandResponseObject, String>;
    fn get_name(&self) -> &str;
    fn get_description(&self) -> &str;
    fn register(&self) -> Vec<CreateApplicationCommandOption> {
        vec![CreateApplicationCommandOption::default()]
    }
    fn get_option_kind(&self) -> CommandOptionType {
        CommandOptionType::SubCommandGroup
    }
}

#[async_trait]
pub trait InteractionResponseHandler {
    async fn handle_interaction_response(&self, data: &MessageComponentInteraction, query_agent: &DBQueryAgent, manager: &DBManager) -> Result<CommandResponseObject, String>;
    fn get_pattern(&self) -> Vec<&str>;
}

#[async_trait]
pub trait ModalSubmitHandler {
    async fn handle_modal_submit(&self, data: &ModalSubmitInteraction, query_agent: &DBQueryAgent, manager: &DBManager) -> Result<CommandResponseObject, String>;
    fn get_pattern(&self) -> Vec<&str>;
}
