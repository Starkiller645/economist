use crate::types::*;
use crate::CommandResponseObject;
use crate::commands::query::*;
use crate::commands::manage::*;
use async_trait::async_trait;
use serenity::model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOptionValue,
    CommandDataOption
};
use serenity::model::prelude::command::CommandOptionType;
use serenity::builder::{CreateComponents, CreateApplicationCommandOption};
use tracing::info;
use crate::utils;


pub struct ListHandler {}

#[async_trait]
impl ApplicationCommandHandler for ListHandler {
    async fn handle_application_command(&mut self, data: &ApplicationCommandInteraction, query_agent: &DBQueryAgent, _manager: &DBManager) -> Result<CommandResponseObject, String> {
        info!("Handling command from `{}`", self.get_name());
        let options = match utils::get_options(data) {
            Ok(o) => o,
            Err(e) => return Err(format!("Error while parsing options: {e:?}"))
        };
        
        let (sort, number) = match self.parse_options(&options) {
            Ok((a, b)) => (a.clone(), b.clone()),
            Err(e) => {
                return Err(e)
            }
        };

        let lookup_result = query_agent.list_currencies(number, sort).await;
        let currencies = match lookup_result {
            Ok(res) => res,
            Err(e) => return Err(format!("{e:?}"))
        };

        let mut currency_desc = "Currency Name";
        let mut code_desc = "Code";
        let mut state_desc = "Nation/State";
        let mut reserve_desc = "Gold Reserves";
        let mut circulation_desc = "Circulation";
        let mut value_desc = "Value";

        match sort {
            CurrencySort::Name => currency_desc = "\u{001b}[1;32mCurrency Name\u{001b}[0m",
            CurrencySort::CurrencyCode => code_desc = "\u{001b}[1;32mCode\u{001b}[0m",
            CurrencySort::State => state_desc = "\u{001b}[1;32mNation/State\u{001b}[0m",
            CurrencySort::Reserves => reserve_desc = "\u{001b}[1;32mGold Reserves\u{001b}[0m",
            CurrencySort::Circulation => circulation_desc = "\u{001b}[1;32mCirculation\u{001b}[0m",
            CurrencySort::Value => value_desc = "\u{001b}[1;32mValue\u{001b}[0m"
        };

        let mut list = "**Currency List**\n```ansi\n┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━┳━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━┓".to_string();
        list += format!("\n┃{code_desc} and {currency_desc}              ┃{state_desc}                  ┃{reserve_desc} ┃{circulation_desc}┃{value_desc}            ┃").as_str();
        list += "\n┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━╋━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━┫";

        // Currency name: 40 chars
        // Currency code: 5 chars (including [])
        // Currency state: 40 chars
        // Reserves: 14 chars
        // Circulation: 14 chars
        // Value: 19 chars
        // Lines: 7 chars
        // Total: 139 chars

        for currency in currencies {
            list += format!(
                "\n┃[\u{001b}[36m{0: <3.3}\u{001b}[0m] \u{001b}[1m{1: <30.30}\u{001b}[0m┃{5: <30.30}┃\u{001b}[1;33m{2: >7.7}\u{001b}[0m ingots┃\u{001b}[1;34m{3: >7.7}\u{001b}[0m {0}┃\u{001b}[1;35m{4: <3.3}\u{001b}[0m {0} / ingot┃",
                currency.currency_code,
                currency.currency_name,
                currency.reserves,
                currency.circulation,
                currency.value,
                currency.state
            ).as_str()
        }

        list += "\n┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━┻━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━┛```";

        Ok(CommandResponseObject::interactive(CreateComponents::default(), list, true))
    }

    fn get_name(&self) -> &str { "list" }

    fn get_option_kind(&self) -> CommandOptionType { CommandOptionType::SubCommand }

    fn get_description(&self) -> &str { "List currencies in circulation, optionally specifying a number and ordering" }

    fn register(&self) -> Vec<CreateApplicationCommandOption> {
        vec![
            CreateApplicationCommandOption::default()
                .kind(CommandOptionType::Integer)
                .name("number")
                .description("Number of currencies to list")
                .clone(),
            CreateApplicationCommandOption::default()
                .kind(CommandOptionType::String)
                .name("sort")
                .description("Attribute to sort currency list by")
                .add_string_choice("Name", "name")
                .add_string_choice("Nation/State", "state")
                .add_string_choice("Currency Code", "code")
                .add_string_choice("Gold Reserves", "reserves")
                .add_string_choice("Circulation", "circulation")
                .add_string_choice("Value", "value")
                .clone()
        ]
    }
}

impl ListHandler {
    pub fn new() -> Self {
        ListHandler {}
    }

    fn parse_options(&self, options: &Vec<CommandDataOption>) -> Result<(CurrencySort, i64), String> {
        let mut sort = CurrencySort::Name;
        let mut number = 10;

        for option in options {
            match option.name.as_str() {
                "sort" => {
                    if let Some(CommandDataOptionValue::String(sort_by)) = option.resolved.clone() {
                        info!("Sorting by '{}'", sort_by.as_str());
                        sort = match sort_by.as_str() {
                            "code" => CurrencySort::CurrencyCode,
                            "name" => CurrencySort::Name,
                            "state" => CurrencySort::State,
                            "reserves" => CurrencySort::Reserves,
                            "circulation" => CurrencySort::Circulation,
                            "value" => CurrencySort::Value,
                            _ => CurrencySort::CurrencyCode
                        };
                    } else {
                        tracing::error!("Couldn't get sort type")
                    }
                },
                "number" => if let Some(CommandDataOptionValue::Integer(num)) = option.resolved {
                    if num <= 0 {
                        return Err("Can't have a negative number of currencies to return.".into())
                    } else {
                        number = num
                    }
                },
                _ => {}
            }
        }

        Ok((sort, number))
    }
}
