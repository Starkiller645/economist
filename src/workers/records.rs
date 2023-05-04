use crate::commands::query::DBQueryAgent;
use crate::commands::manage::DBManager;
use tracing::{info, warn, error};
use sqlx::postgres::PgPool;
use shuttle_persist::PersistInstance;
use chrono::{NaiveTime, offset::Utc};
use crate::types::*;
use futures::channel::mpsc;
use std::collections::HashMap;
use tokio::time::sleep;

pub async fn record_worker(_persist: PersistInstance, pool: PgPool, mut rx: mpsc::Receiver<WorkerMessage>) {
    info!("Starting records worker...");
    /*let mut last_date: DateTime<Utc> = match persist.load("last-record-time") {
        Ok(datetime) => datetime,
        Err(e) => {
            warn!("Couldn't load persisted object 'last-record-time': {e:?}");
            Utc::now()
        }
    };

    let time_now = Utc::now();

    if time_now.signed_duration_since(last_date).num_hours() >= 12 {
        record_update(pool).await;
    }*/

    let mut open = false;

    let opening_time = NaiveTime::from_hms_opt(6, 0, 0).unwrap();
    let closing_time = NaiveTime::from_hms_opt(18, 0, 0).unwrap();

    let mut opening_data: HashMap<i64, CurrencyData> = HashMap::new();
    let mut closing_data: HashMap<i64, CurrencyData> = HashMap::new();

    let query_agent = DBQueryAgent::new(pool.clone());
    let manager = DBManager::new(pool);

    loop {
        if let Ok(Some(message)) = rx.try_next() {
            match message {
                WorkerMessage::Halt => {
                    warn!("Halting worker 'record'...");
                    rx.close();
                    return
                }
            }
        }
        let now = Utc::now().time();
        if now > opening_time && now < closing_time && !open {
            open = true;
            match query_agent.list_currencies(200, crate::commands::query::CurrencySort::CurrencyCode).await {
                Ok(data) => {
                    for currency in data {
                        opening_data.insert(currency.currency_id, currency);
                    };
                    info!("Logged data at opening!");
                    info!("Time now: {now:?}");
                },
                Err(e) => {
                    error!("Couldn't get currency data at opening: {e:?}");
                    continue;
                }
            }
        }

        if now > closing_time && open {
            open = false;
            match query_agent.list_currencies(200, crate::commands::query::CurrencySort::CurrencyCode).await {
                Ok(data) => {
                    for currency in data {
                        closing_data.insert(currency.currency_id, currency);
                    };
                    info!("Logged data at closing!");
                    info!("Time now: {now:?}");
                }
                Err(e) => {
                    error!("Couldn't get currency data at closing: {e:?}");
                    continue;
                }
            };

            for (_id, currency) in opening_data.clone() {
                if !closing_data.contains_key(&currency.currency_id) {
                    continue
                }
                let return_data = match manager.insert_record(currency.currency_id, currency.value, closing_data.get(&currency.currency_id).unwrap().value).await {
                    Ok(data) => data,
                    Err(e) => {
                        error!("Couldn't get result of insert command: error: {e:?}");
                        continue;
                    }
                };
                info!("Inserted new record: {return_data:#?}");
            }
        };
        sleep(chrono::Duration::seconds(10).to_std().unwrap()).await;
    }
}
