use crate::commands::currency::{CurrencyData, TransactionData};
use sqlx::{Row, postgres::PgPool};
use chrono::offset::Utc;

#[derive(Clone)]
pub struct DBManager {
    pool: PgPool
}

impl DBManager {
    pub fn new(pool: PgPool) -> Self {
        DBManager {
            pool
        }
    }

    pub async fn add_currency(&self, currency_code: String, currency_name: String, circulation: i64, gold_reserve: i64, state: String) -> Result<CurrencyData, sqlx::Error> {
        match sqlx::query("INSERT INTO currencies(currency_code, currency_name, circulation, reserves, state) VALUES ($1, $2, $3, $4, $5) RETURNING currency_id;")
            .bind(currency_code.clone())
            .bind(currency_name.clone())
            .bind(circulation)
            .bind(gold_reserve)
            .bind(state.clone())
            .fetch_one(&self.pool).await {
                Ok(row) => {
                    let currency_id = row.try_get("currency_id")?;
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

    pub async fn remove_currency(&self, currency_code: String) -> Result<(), sqlx::Error> {
        match sqlx::query("DELETE FROM currencies WHERE currency_code = $1;")
            .bind(currency_code)
            .execute(&self.pool)
            .await {
                Ok(_) => Ok(()),
                Err(e) => return Err(e)    
            }
    }

    pub async fn reserve_add(&self, currency_code: String, amount: i64) -> Result<TransactionData, sqlx::Error> {
        let currency_id: i64 = match sqlx::query("SELECT currency_id FROM currencies WHERE currency_code = $1")
            .bind(currency_code.clone())
            .fetch_one(&self.pool).await {
                Ok(row) => {
                    match row.try_get("currency_id") {
                        Ok(id) => id,
                        Err(e) => return Err(e)
                    }
                },
                Err(e) => return Err(e)
            };
        
        let transaction_date = Utc::now();

        let transaction_id: i64 = match sqlx::query("INSERT INTO transactions(transaction_date, currency_id, delta_reserves) VALUES ($1, $2, $3) RETURNING transaction_id")
            .bind(transaction_date)
            .bind(currency_id)
            .bind(amount)
            .fetch_one(&self.pool)
            .await {
                Ok(row) => {
                    match row.try_get("transaction_id") {
                        Ok(id) => id,
                        Err(e) => return Err(e)
                    }
                },
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

    pub async fn get_currency_data(&self, currency_code: String) -> Result<CurrencyData, sqlx::Error> {
        match sqlx::query_as("SELECT * FROM currencies WHERE currency_code = $1")
            .bind(currency_code)
            .fetch_one(&self.pool)
            .await {
                Ok(row) => Ok(row),
                Err(e) => Err(e)
            }
    }
}
