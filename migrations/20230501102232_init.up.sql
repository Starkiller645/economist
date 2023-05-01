-- Add up migration script here
CREATE TABLE IF NOT EXISTS currencies(
	currency_id BIGSERIAL NOT NULL,
	currency_code TEXT NOT NULL UNIQUE,
	currency_name TEXT NOT NULL,
	state TEXT NOT NULL,
	circulation BIGINT NOT NULL,
	reserves BIGINT NOT NULL,
	PRIMARY KEY (currency_id)
);

CREATE TABLE IF NOT EXISTS transactions(
	transaction_id BIGSERIAL NOT NULL,
	transaction_date DATE NOT NULL,
	currency_id BIGINT NOT NULL,
	delta_circulation BIGINT,
	delta_reserves BIGINT,
	PRIMARY KEY (transaction_id),
	FOREIGN KEY (currency_id) REFERENCES currencies(currency_id) ON DELETE CASCADE
);
