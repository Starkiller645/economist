use crate::get_sql_connection;
use crate::commands::currency::{CurrencyData, TransactionData};
use sqlx::Row;
use chrono::offset::Utc;

pub async fn add_currency(currency_code: String, currency_name: String, circulation: i64, gold_reserve: i64, state: String) -> Result<CurrencyData, sqlx::Error> {
    let mut sql_conn = get_sql_connection().await?;
    match sqlx::query("INSERT INTO currencies(currency_code, currency_name, circulation, reserves, state) VALUES (?, ?, ?, ?, ?)")
        .bind(currency_code.clone())
        .bind(currency_name.clone())
        .bind(circulation)
        .bind(gold_reserve)
        .bind(state.clone())
        .execute(&mut sql_conn).await {
            Ok(_conn) => {
                let currency_id_row = sqlx::query("SELECT LAST_INSERT_ID() AS currency_id")
                    .fetch_one(&mut sql_conn)
                    .await?;
                let currency_id: u64 = currency_id_row.try_get("currency_id")?;
                Ok(CurrencyData {
                    currency_id,
                    currency_name,
                    currency_code,
                    circulation,
                    reserves: gold_reserve,
                    state
                })   
            },
            Err(e) => Err(e)
        }
}

pub async fn remove_currency(currency_code: String) -> Result<(), sqlx::Error> {
    let mut sql_conn = get_sql_connection().await?;
    match sqlx::query("DELETE FROM currencies WHERE currency_code = ?;")
        .bind(currency_code)
        .execute(&mut sql_conn)
        .await {
            Ok(_) => Ok(()),
            Err(e) => return Err(e)    
        }
}

pub async fn reserve_add(currency_code: String, amount: i64) -> Result<TransactionData, sqlx::Error> {
    let mut sql_conn = get_sql_connection().await?;
    let currency_id: u64 = match sqlx::query("SELECT currency_id FROM currencies WHERE currency_code = ?")
        .bind(currency_code.clone())
        .fetch_one(&mut sql_conn).await {
            Ok(row) => {
                match row.try_get("currency_id") {
                    Ok(id) => id,
                    Err(e) => return Err(e)
                }
            },
            Err(e) => return Err(e)
        };
    
    let transaction_date = Utc::now();

    match sqlx::query("INSERT INTO transactions(transaction_date, currency_id, delta_reserves) VALUES (?, ?, ?)")
        .bind(transaction_date)
        .bind(currency_id)
        .bind(amount)
        .execute(&mut sql_conn)
        .await {
            Ok(_) => {},
            Err(e) => return Err(e)
        };

    let transaction_id: u64 = match sqlx::query("SELECT LAST_INSERT_ID() AS transaction_id")
        .fetch_one(&mut sql_conn)
        .await {
            Ok(num) => {
                match num.try_get("transaction_id") {
                    Ok(num) => num,
                    Err(e) => return Err(e)
                }
            }
            Err(e) => return Err(e)
        };

    Ok(TransactionData {
        transaction_id,
        transaction_date,
        currency_code,
        delta_reserves: Some(amount),
        delta_circulation: None
    })
}

pub async fn get_currency_data(currency_code: String) -> Result<CurrencyData, sqlx::Error> {
    let mut sql_conn = get_sql_connection().await?;
    match sqlx::query_as("SELECT * FROM currencies WHERE currency_code = ?")
        .bind(currency_code)
        .fetch_one(&mut sql_conn)
        .await {
            Ok(row) => Ok(row),
            Err(e) => Err(e)
        }
}
