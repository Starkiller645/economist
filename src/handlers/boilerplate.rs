use crate::commands::manage::*;
use crate::commands::query::*;
use crate::types::*;
use crate::CommandResponseObject;
use async_trait::async_trait;
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
};

pub struct BoilerplateHandler {}

#[async_trait]
impl ApplicationCommandHandler for BoilerplateHandler {
    async fn handle_application_command(&mut self, data: &ApplicationCommandInteraction, query_agent: &DBQueryAgent, manager: &DBManager) -> Result<CommandResponseObject, String> {
        Ok(CommandResponseObject::text(""))
    }

    fn get_name(&self) -> &str { "boilerplate" }
    fn get_description(&self) -> &str { "This is a boilerplate for implementing ApplicationCommandHandler" }
    fn register(&self) -> Vec<CreateApplicationCommandOption> {
        vec![]
    }
}

#[async_trait]
impl InteractionResponseHandler for BoilerplateHandler {
    async fn handle_interaction_response(&self, data: &MessageComponentInteraction, query_agent: &DBQueryAgent, manager: &DBManager) -> Result<CommandResponseObject, String> {
        Ok(CommandResponseObject::text(""))
    }

    fn get_pattern(&self) -> Vec<&str> {
        vec![]
    }
}
