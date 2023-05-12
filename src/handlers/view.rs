use crate::commands::manage::*;
use crate::commands::query::*;
use crate::types::*;
use crate::utils;
use crate::CommandResponseObject;
use async_trait::async_trait;
use tracing::info;
use serenity::builder::CreateApplicationCommandOption;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
};

pub struct ViewHandler {}

#[async_trait]
impl ApplicationCommandHandler for ViewHandler {
    async fn handle_application_command(&mut self, data: &ApplicationCommandInteraction, query_agent: &DBQueryAgent, _manager: &DBManager) -> Result<CommandResponseObject, String> {
        info!("{data:?}");
        let options = match utils::get_options(data) {
            Ok(o) => o,
            Err(e) => return Err(format!("Error while parsing options: {e:?}"))
        };

        let code = match self.parse_options(&options) {
            Ok(c) => c,
            Err(e) => return Err(e)
        };

        let currency_data = match query_agent.get_currency_data(code.clone()).await {
            Ok(d) => d,
            Err(e) => return Err(format!("Error getting currency data: {e:?}"))
        };

        let records = match query_agent.get_reports(1, code).await {
            Ok(r) => r,
            Err(e) => return Err(format!("Error getting records: {e:?}"))
        };

        let mut embed = serenity::builder::CreateEmbed::default()
            .title(format!("{}", currency_data.currency_name))
            .clone();

        let mut description = format!(
                "> Nation/State: _{0}_\n> Reserves: `{1} ingots`\n> Circulation: `{2} {3}`\n> Value: `{4:.3} ingot / {3}`",
                currency_data.state,
                currency_data.reserves,
                currency_data.circulation,
                currency_data.currency_code,
                currency_data.value
            );

        match records.get(0) {
            Some(record) => {
                let record_id = record.record_id;
                info!("Using url https://economist-image-server.shuttleapp.rs/{:05}/{:05}", currency_data.currency_id, record_id);

                embed = embed
                    .image(format!("https://economist-image-server.shuttleapp.rs/{:05}/{:05}", currency_data.currency_id, record_id))
                    .clone();
            },
            None => {
                description += "\n```ansi\n\u{001b}[1;33mWarning:\u{001b}[0m No past records available for this currency```"
            }
        };

        embed = embed
            .description(description)
            .clone();

        Ok(
            CommandResponseObject::embed(
                embed.clone()
            )
        )
    }

    fn get_name(&self) -> &str { "view" }
    fn get_description(&self) -> &str { "View detailed information about a currency" }
    fn get_option_kind(&self) -> CommandOptionType { CommandOptionType::SubCommand }
    fn register(&self) -> Vec<CreateApplicationCommandOption> {
        vec![
            CreateApplicationCommandOption::default()
                .kind(CommandOptionType::String)
                .name("code")
                .description("Three-letter currency code to view")
                .required(true)
                .clone()
        ]
    }
}

impl ViewHandler {
    pub fn new() -> Self {
        ViewHandler {}
    }

    fn parse_options(&self, options: &Vec<CommandDataOption>) -> Result<String, String> {
        let mut code = None;
        for option in options {
            match option.name.as_str() {
                "code" => {
                    info!("{:?}", option.resolved.clone());
                    if let Some(CommandDataOptionValue::String(c_code)) = option.resolved.clone() {
                        code = Some(c_code);
                    }
                }
                _ => {}
            }
        }

        match code {
            Some(code) => Ok(code),
            None => Err("Couldn't get code from options".into())
        }
    }
}
