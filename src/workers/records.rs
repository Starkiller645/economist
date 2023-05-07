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
use plotters::prelude::*;

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
    let closing_time = NaiveTime::from_hms_opt(18, 0, 30).unwrap();

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
                let record = match manager.insert_record(currency.currency_id, currency.value, closing_data.get(&currency.currency_id).unwrap().value).await {
                    Ok(data) => {
                        info!("Inserted record for {currency:?}");
                        data
                    },
                    Err(e) => {
                        error!("Couldn't get result of insert command: error: {e:?}");
                        continue;
                    }
                };

                match query_agent.get_reports(14, currency.currency_code.clone()).await {
                    Ok(data) => {
                        let filename = format!("data/{:05}.png", currency.currency_id);
                        {
                            //let root = BitMapBackend::with_buffer(&mut buffer, (1024, 768)).into_drawing_area();
							let root = BitMapBackend::new(filename.as_str(), (1024, 768)).into_drawing_area();
                            let bg_color = RGBColor(56, 58, 64);
                            root.fill(&bg_color).unwrap();
                            let (to_date, from_date) = (
                                data.get(0).unwrap().record_date,
                                data.get(data.len() - 1).unwrap().record_date
                            );

                            let mut max_value: f64 = 0.0;
                            for record in data.clone() {
                                if record.closing_value > max_value { max_value = record.closing_value }
                            }

                            max_value += 1.0;

                            let mut chart = ChartBuilder::on(&root)
                                .margin(10)
                                .caption(format!("Currency trend for {}", currency.currency_name), ("sans-serif", 40, &WHITE))
                                .set_label_area_size(LabelAreaPosition::Left, 60)
                                .set_label_area_size(LabelAreaPosition::Right, 60)
                                .set_label_area_size(LabelAreaPosition::Bottom, 40)
                                .build_cartesian_2d(from_date..to_date, 0f64..max_value)
                                .unwrap();

                            chart
                                .configure_mesh()
                                .disable_x_mesh()
                                .disable_y_mesh()
                                .x_labels(30)
                                .max_light_lines(4)
                                .y_desc(format!("Value in {} / gold ingot", currency.currency_code))
                                .axis_desc_style(("sans-serif", 30, &WHITE))
                                .x_label_style(("sans-serif", 20, &WHITE))
                                .y_label_style(("sans-serif", 20, &WHITE))
                                .axis_style(&WHITE)
                                .draw()
                                .unwrap();


                            chart
                                .draw_series(
                                    LineSeries::new(
                                    data.iter().map(|record| {
                                        (record.record_date, record.closing_value)
                                    }), 
                                    &BLUE
                                    )
                                )
                                .unwrap();

                            root.present().expect("Error generating graph!");
                        }

                        let client = reqwest::Client::new();
                        let file_stream = std::fs::read(filename.clone()).unwrap();
                        let file_part = reqwest::multipart::Part::bytes(file_stream)
                            .file_name(filename)
                            .mime_str("image/jpg")
                            .unwrap();
                        let form = reqwest::multipart::Form::new()
                            .part("file", file_part);

                        let req = client.
                            post(format!("https://economist-image-server.shuttleapp.rs/{:05}/{:05}", currency.currency_id, record.record_id))
                            .multipart(form);
                        warn!("Generated request {req:?}");
                        let res = req
                            .send()
                            .await;
                            
                        info!("Send HTTP POST, got response: {res:?}");
                    }
                    Err(e) => warn!("Caught an error while looking up records for currency `{}`: {e}", currency.currency_code)
                }

            }
            info!("Logged records!");
        };
        sleep(chrono::Duration::seconds(10).to_std().unwrap()).await;
    }
}
