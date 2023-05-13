use crate::types::*;
use sqlx::{Row, postgres::PgPool};
use chrono::offset::Utc;

#[derive(Clone)]
pub struct DBManager {
    pool: PgPool
}

pub enum ModifyMetaType {
    Name,
    Code,
    State
}

impl DBManager {
    pub fn new(pool: PgPool) -> Self {
        DBManager {
            pool
        }
    }

    pub async fn add_currency(&self, currency_code: String, currency_name: String, circulation: i64, gold_reserve: i64, state: String, owner: String) -> Result<CurrencyData, sqlx::Error> {
        match sqlx::query("INSERT INTO currencies(currency_code, currency_name, circulation, reserves, state, owner) VALUES ($1, $2, $3, $4, $5, $6) RETURNING currency_id;")
            .bind(currency_code.clone())
            .bind(currency_name.clone())
            .bind(circulation)
            .bind(gold_reserve)
            .bind(state.clone())
            .bind(owner.clone())
            .fetch_one(&self.pool).await {
                Ok(row) => {
                    let currency_id = row.try_get("currency_id")?;
                    Ok(CurrencyData {
                        currency_id,
                        currency_name,
                        currency_code,
                        circulation,
                        value: gold_reserve as f64 / circulation as f64,
                        reserves: gold_reserve,
                        state,
                        owner
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

    pub async fn reserve_modify(&self, currency_code: String, amount: i64, initiator: String) -> Result<TransactionData, sqlx::Error> {
        let currency_data: CurrencyData = match sqlx::query_as("SELECT * FROM currencies WHERE currency_code = $1")
            .bind(currency_code.clone())
            .fetch_one(&self.pool).await {
                Ok(row) => row,
                Err(e) => return Err(e)
            };
        
        let transaction_date = Utc::now();
        let prev_reserves = currency_data.reserves;
        let new_reserves = prev_reserves + amount;

        let transaction_id: i64 = match sqlx::query("INSERT INTO transactions(transaction_date, currency_id, delta_reserves, initiator) VALUES ($1, $2, $3, $4) RETURNING transaction_id")
            .bind(transaction_date)
            .bind(currency_data.currency_id)
            .bind(amount)
            .bind(initiator)
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

        match sqlx::query("UPDATE currencies SET reserves = $1 WHERE currency_id = $2")
            .bind(new_reserves)
            .bind(currency_data.currency_id)
            .execute(&self.pool)
            .await {
                Ok(_) => {},
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

    pub async fn circulation_modify(&self, currency_code: String, amount: i64, initiator: String) -> Result<TransactionData, sqlx::Error> {
        let currency_data: CurrencyData = match sqlx::query_as("SELECT * FROM currencies WHERE currency_code = $1")
            .bind(currency_code.clone())
            .fetch_one(&self.pool).await {
                Ok(row) => row,
                Err(e) => return Err(e)
            };
        
        let transaction_date = Utc::now();
        let prev_circulation = currency_data.circulation;
        let new_circulation = prev_circulation + amount;

        let transaction_id: i64 = match sqlx::query("INSERT INTO transactions(transaction_date, currency_id, delta_circulation, initiator) VALUES ($1, $2, $3, $4) RETURNING transaction_id")
            .bind(transaction_date)
            .bind(currency_data.currency_id)
            .bind(amount)
            .bind(initiator)
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

        match sqlx::query("UPDATE currencies SET circulation = $1 WHERE currency_id = $2")
            .bind(new_circulation)
            .bind(currency_data.currency_id)
            .execute(&self.pool)
            .await {
                Ok(_) => {},
                Err(e) => return Err(e)
            };

        Ok(TransactionData {
            transaction_id,
            transaction_date,
            currency_code,
            delta_reserves: None,
            delta_circulation: Some(amount)
        })
    }

    pub async fn modify_currency_meta(&self, currency_code: String, kind: ModifyMetaType, data: String) -> Result<CurrencyData, sqlx::Error> {
        let sql_result = sqlx::query_as(format!("UPDATE currencies SET {} = $1 WHERE currency_code = $2 RETURNING *", match kind {
                ModifyMetaType::Name => "currency_name",
                ModifyMetaType::Code => "currency_code",
                ModifyMetaType::State => "state"
            }).as_str())
            .bind(data)
            .bind(currency_code)
            .fetch_one(&self.pool).await?;

        Ok(sql_result)
    }

    pub async fn danger_recreate_database(&self) -> Result<(), sqlx::Error> {
        match sqlx::query("DROP TABLE IF EXISTS transactions;")
            .execute(&self.pool)
            .await {
                Ok(_) => {},
                Err(e) => return Err(e)
            };
        match sqlx::query("DROP TABLE IF EXISTS records;")
            .execute(&self.pool)
            .await {
                Ok(_) => {},
                Err(e) => return Err(e)
            }
        match sqlx::query("DROP TABLE IF EXISTS currencies;")
            .execute(&self.pool)
            .await {
                Ok(_) => {},
                Err(e) => return Err(e)
            };
        match sqlx::query("CREATE TABLE IF NOT EXISTS currencies(
        currency_id BIGSERIAL NOT NULL,
        currency_code TEXT NOT NULL UNIQUE,
        currency_name TEXT NOT NULL,
        state TEXT NOT NULL,
        circulation BIGINT NOT NULL,
        reserves BIGINT NOT NULL,
        owner TEXT NOT NULL,
        value DOUBLE PRECISION GENERATED ALWAYS AS (
            CASE WHEN reserves <= 0 THEN 0
                 WHEN circulation <= 0 THEN 0 
                 ELSE (
                     CAST(reserves AS DOUBLE PRECISION) / CAST(circulation AS DOUBLE PRECISION)
                 ) 
                 END
            ) STORED,
        PRIMARY KEY (currency_id)
    );")
            .execute(&self.pool)
            .await {
                Ok(_) => {},
                Err(e) => return Err(e)
            };
        match sqlx::query("CREATE TABLE transactions(
            transaction_id BIGSERIAL NOT NULL,
            transaction_date DATE NOT NULL,
            currency_id BIGINT NOT NULL,
            delta_circulation BIGINT,
            delta_reserves BIGINT,
            initiator TEXT NOT NULL,
            PRIMARY KEY (transaction_id),
            FOREIGN KEY (currency_id) REFERENCES currencies(currency_id) ON DELETE CASCADE
        );")
            .execute(&self.pool)
            .await {
                Ok(_) => {},
                Err(e) => return Err(e)
            };

        match sqlx::query("CREATE TABLE IF NOT EXISTS records(
        record_id BIGSERIAL NOT NULL,
        record_date DATE NOT NULL,
        currency_id BIGINT NOT NULL,
        opening_value DOUBLE PRECISION,
        closing_value DOUBLE PRECISION,
        delta_value DOUBLE PRECISION GENERATED ALWAYS AS (closing_value - opening_value) STORED,
        growth SMALLINT GENERATED ALWAYS AS (
            CASE WHEN (closing_value - opening_value) = 0 THEN 0
                 WHEN (closing_value - opening_value) > 0 THEN 1
                 ELSE -1
                 END
            ) STORED,
        PRIMARY KEY (record_id),
        FOREIGN KEY (currency_id) REFERENCES currencies(currency_id) ON DELETE CASCADE
        )
    ").execute(&self.pool).await {
        Ok(_) => {},
        Err(e) => return Err(e)
    };
        Ok(())
    }

    pub async fn insert_record(&self, currency_id: i64, opening_value: f64, closing_value: f64) -> Result<RecordData, sqlx::Error> {
        let todays_date: chrono::NaiveDate = Utc::now().date_naive();
        sqlx::query_as("INSERT INTO records(record_date, currency_id, opening_value, closing_value) VALUES ($1, $2, $3, $4) RETURNING *")
            .bind(todays_date)
            .bind(currency_id)
            .bind(opening_value)
            .bind(closing_value)
            .fetch_one(&self.pool).await
    }
}
