use crate::commands::currency::{CurrencyData, TransactionData};
use sqlx::{Row, postgres::PgPool};
use chrono::offset::Utc;
use futures::TryStreamExt;

#[derive(Clone)]
pub struct DBQueryAgent {
    pool: PgPool
}

impl DBQueryAgent {
    pub fn new(pool: PgPool) -> Self {
        DBQueryAgent {
            pool
        }
    }
}

pub enum CurrencySort {
    Name,
    CurrencyCode,
    Reserves,
    Circulation,
    Value,
    State
}

impl DBQueryAgent {
    pub async fn get_currency_data(&self, currency_code: String) -> Result<CurrencyData, sqlx::Error> {
        match sqlx::query_as("SELECT * FROM currencies WHERE currency_code = $1")
            .bind(currency_code)
            .fetch_one(&self.pool)
            .await {
                Ok(row) => Ok(row),
                Err(e) => Err(e)
            }
    }
    
    pub async fn get_transaction_data(&self, transaction_id: i64) -> Result<TransactionData, sqlx::Error> {
        match sqlx::query_as("SELECT * FROM transactions WHERE transaction_id = $1")
            .bind(transaction_id)
            .fetch_one(&self.pool)
            .await {
                Ok(row) => Ok(row),
                Err(e) => Err(e)
            }
    }

    pub async fn list_currencies(&self, number: i64, sort: CurrencySort) -> Result<Vec<CurrencyData>, sqlx::Error> {

        let order_by = match sort {
            CurrencySort::Name => "currency_name",
            CurrencySort::CurrencyCode => "currency_code",
            CurrencySort::State => "state",
            CurrencySort::Reserves => "reserves",
            CurrencySort::Circulation => "circulation",
            CurrencySort::Value => "value"
        };

        println!("ORDERING BY: {order_by}");
        let query = format!("SELECT * FROM currencies ORDER BY {}", order_by);

        let mut stream = sqlx::query_as::<_, CurrencyData>(query.as_str())
            .fetch(&self.pool);

        let mut currency_vec = vec![];
        let mut i = 0;

        while let Some(data) = stream.try_next().await? {
            if i == number {
                return Ok(currency_vec)
            }
            match sort {
                CurrencySort::Reserves | CurrencySort::Circulation | CurrencySort::Value => currency_vec.insert(0, data),
                _ => currency_vec.push(data)
            }
            i += 1;
        }
        Ok(currency_vec)
    }
}