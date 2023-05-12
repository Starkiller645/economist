use crate::commands::manage::*;
use crate::commands::query::*;
use crate::types::*;
use crate::utils;
use crate::CommandResponseObject;
use async_trait::async_trait;
use serenity::model::prelude::command::CommandOptionType;
use serenity::builder::{CreateComponents, CreateApplicationCommandOption};
use serenity::model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
};

pub struct RecordsHandler {}

#[async_trait]
impl ApplicationCommandHandler for RecordsHandler {
    async fn handle_application_command(&mut self, data: &ApplicationCommandInteraction, query_agent: &DBQueryAgent, _manager: &DBManager) -> Result<CommandResponseObject, String> {

        let options = match utils::get_options(&data) {
            Ok(o) => o,
            Err(e) => return Err(format!("Error while getting options from command data: {e:?}"))
        };

        let (number, currency_code) = match self.parse_options(&options) {
            Ok(n) => n,
            Err(e) => return Err(format!("Error while parsing options: {e:?}"))
        };

		let currency = match query_agent.get_currency_data(currency_code.clone()).await {
			Ok(c) => c,
			Err(e) => return Err(format!("Error while getting currency data: {e:?}"))
		};

		let records = match query_agent.get_reports(number, currency_code.clone()).await {
			Ok(r) => r,
			Err(e) => return Err(format!("Error while looking up currency records: {e:?}"))
		};

		let currency_string = format!("[\u{001b}[36m{0}\u{001b}[0m] \u{001b}[1m{1}\u{001b}[0m\n", currency.currency_code, currency.currency_name);

		let mut final_string = format!("```ansi\nRecord list for {}\n", currency_string);
        final_string += "┏━━━━━━━━━━┳━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━┓\n";
        final_string += "┃Date      ┃Value at Opening ┃Value at Closing ┃Change in Value ┃Performance   ┃\n";
        final_string += "┣━━━━━━━━━━╋━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━┫";

		for record in records {
            let performance_description = if record.growth == 0 {
                    "Holding Steady"
                } else if record.growth < 0 {
                    "In Decline"
                } else {
                    "Gaining Value"
                };
            let performance_color = if record.growth == 0 {
                    "\u{001b}[1m"
                } else if record.growth < 0 {
                    "\u{001b}[1;31m"
                } else {
                    "\u{001b}[1;32m"
                };
			final_string += format!(
				"\n┃{0: <10.10}┃\u{001b}[1;35m{1: <5.3}\u{001b}[0m ingot / {5}┃\u{001b}[1;35m{2: <5.3}\u{001b}[0m ingot / {5}┃{performance_color}{3: <16.3}\u{001b}[0m┃{performance_color}{4: <14.14}\u{001b}[0m┃",
                record.record_date,
                record.opening_value,
                record.closing_value,
                record.delta_value,
                performance_description,
                currency.currency_code
			).as_str()
		};
		// Currency name: 40 chars
		// Date: 10 chars
		// Opening and Closing Values; 8 chars
		// Change in value: 9 chars
		// Growth: 'In Decline' 'Holding Steady' 'Gaining Value' 14 chars
        
        final_string += "\n┗━━━━━━━━━━┻━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━┛```";

        Ok(CommandResponseObject::interactive(CreateComponents::default(), final_string, true))
    }

    fn get_name(&self) -> &str { "records" }
    fn get_description(&self) -> &str { "View past currency end-of-day records" }
	fn get_option_kind(&self) -> CommandOptionType { CommandOptionType::SubCommand }
    fn register(&self) -> Vec<CreateApplicationCommandOption> {
        vec![
            CreateApplicationCommandOption::default()
                .kind(CommandOptionType::String)
                .name("code")
                .description("Three-letter currency code to view records for")
                .required(true)
                .max_length(3)
                .min_length(3)
                .clone(),
            CreateApplicationCommandOption::default()
                .kind(CommandOptionType::Integer)
                .name("number")
                .description("Maximum number of records to fetch")
                .clone()
        ]
    }
}

impl RecordsHandler {
    pub fn new() -> Self {
        RecordsHandler {}
    }

    fn parse_options(&self, options: &Vec<CommandDataOption>) -> Result<(i64, String), String> {
        let mut number = 10;
		let mut currency_code = String::new();

        for option in options {
            match option.name.as_str() {
                "number" => { if let Some(CommandDataOptionValue::Integer(n)) = option.resolved.clone() { 
					if n > 0 {
						number = n;
					} else {
						return Err("Number cannot be less than or equal to zero".into())
					}
                }},
				"code" => { if let Some(CommandDataOptionValue::String(c)) = option.resolved.clone() {
					currency_code = c;
				}} 
                _ => {}
            }
        };

        Ok((number, currency_code))
    }
}
