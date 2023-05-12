use crate::commands::manage::*;
use crate::commands::query::*;
use crate::types::*;
use crate::utils;
use crate::CommandResponseObject;
use async_trait::async_trait;
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::model::application::component::ButtonStyle;
use serenity::builder::{CreateComponents, CreateApplicationCommandOption};
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;

pub struct DatabaseHandler {}

#[async_trait]
impl ApplicationCommandHandler for DatabaseHandler {
    async fn handle_application_command(&mut self, data: &ApplicationCommandInteraction, _query_agent: &DBQueryAgent, _manager: &DBManager) -> Result<CommandResponseObject, String> {
        let options = match utils::get_options(&data) {
            Ok(o) => o,
            Err(e) => return Err(format!("Error while getting options from command data: {e:?}"))
        };

        let action = match options.get(0) {
            Some(a) => a,
            None => return Err(format!("Error while parsing options: Couldn't get subcommand data"))
        };

        match action.name.as_str() {
            "recreate" => {
                Ok(CommandResponseObject::interactive(CreateComponents::default()
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
                    "***DANGER***\nThis command ***CANNOT BE REVERSED***\nIf you click confirm, you will lose access to:\n- **All currencies, their metadata, circulation amount and reserves**\n- **All transaction history, for all currencies, forever**\n- **All current and past currency values, against their gold reserves and each other**\nOnly use this command if you _absolutely_ know what you are doing\nAre you _100% sure_ you want to continue?",
                    true
                    )
                )
            },
            _ => {
                Err("Error: couldn't find the requested subcommand".into())
            }
        }
    }

    fn get_name(&self) -> &str { "database" }
    fn get_description(&self) -> &str { "Manage and recreate the Economist Bot database" }
    fn register(&self) -> Vec<CreateApplicationCommandOption> {
        vec![
            CreateApplicationCommandOption::default()
                .kind(CommandOptionType::SubCommand)
                .name("recreate")
                .description("Recreate the entire currency database, starting from scratch. DANGER, THIS IS NOT REVERSIBLE!!!")
                .clone()
        ]
    }
}

#[async_trait]
impl InteractionResponseHandler for DatabaseHandler {
    async fn handle_interaction_response(&self, data: &MessageComponentInteraction, _query_agent: &DBQueryAgent, manager: &DBManager) -> Result<CommandResponseObject, String> {

        Ok(match data.data.custom_id.as_str() {
            "recreate-database-confirm" => match manager.danger_recreate_database().await {
                    Ok(_) => CommandResponseObject::interactive_with_feedback(CreateComponents::default(), "Database successfully recreated", format!("{0} recreated the Economist Bot database. All stored data has been lost.", data.user), true),
                    Err(e) => return Err(format!("Error recreating database (this is probably a good thing): {e:?}"))
                }

            "recreate-database-cancel" => CommandResponseObject::interactive_with_feedback(CreateComponents::default(), "Cancelled deleting database (this is probably a good thing)", "", true),
            _ => return Err("Error: unknown custom id".into())
        })
    }

    fn get_pattern(&self) -> Vec<&str> {
        vec!["recreate-database-confirm", "recreate-database-cancel"]
    }
}

impl DatabaseHandler {
    pub fn new() -> Self {
        DatabaseHandler {}
    }
}
