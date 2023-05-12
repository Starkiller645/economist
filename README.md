<div align="center">

![Economist Bot Logo](../assets/logo-wordmark.png)

# Economist Bot
A Discord bot to assist in managing a complete virtual economy

![GPL License Badge](https://img.shields.io/github/license/Starkiller645/economist?color=%23ffcb21&style=for-the-badge)
![Version Badge](https://img.shields.io/github/v/tag/Starkiller645/economist?color=%23219aff&include_prereleases&label=Version&style=for-the-badge)
![Lines of Code Badge](https://img.shields.io/tokei/lines/github/Starkiller645/economist?color=%23219aff&style=for-the-badge)
![CI Build Badge](https://img.shields.io/github/actions/workflow/status/starkiller645/economist/rust.yml?logo=github&style=for-the-badge)

</div>

## :information_source: About
Economist Bot allows users to create, modify and delete currencies, backed by their federal gold reserves. While this bot is designed for Minecraft servers, the concept works for managing any kind of virtual currency. The bot will log records of each transaction, as well as performance of each currency at the close of trading each day, and generate graphs of each currency's performance over time.

## :question: How to Use
Economist Bot assumes a currency system set up in the following way:
- One or many currencies, with a known amount of every currency in circulation
- Each currency is backed by a known federal reserve of gold

This way, the value of a currency is dependant on both its current reserves, and the amount in circulation, preventing currency value from being an arbitrary number. The caveat to this is that careful management of gold reserves and circulation is required to maintain a stable currency. Too much in circulation (or too little in the reserves), and the currency will crash - likewise, too little in circulation will result in the currency becoming unobtainable as its value is massively inflated.  
  
The only thing that you are required to do to effectively use this bot is log, via commands, every time either the circulation or reserve changes. The bot will automatically generate current values, records of performance and transactions, and graphs of performance over time, without any additional input.

## :sparkles: Features
- [x] Create and delete currencies
- [x] Add and remove gold reserves and currency in circulation
- [x] Modify currency metadata (name, three-letter code &c.)
- [x] List and sort currencies
- [x] View current currency data and performance graph
- [x] Log and view end-of-day records
- [ ] Compare currencies to each other (forex)
- [ ] List previous currency transactions
- [ ] Add stocks to the bot

## :construction: Building
This bot is built with Rust and uses [Shuttle](https://shuttle.rs/) for hosting. This means that to self-host your own version of this bot, you need to install rustc, Cargo and Shuttle's CLI.  

### Dependencies
- `cargo & rustc ^1.68.2`
- `cargo-shuttle 0.15.0`
- `tokio ^1.28.0`
- `serenity ^0.11.5`
- `sqlx ^0.6.3`
- `reqwest ^0.11.17`
- `tracing`
- `anyhow`
- Other minor dependencies are specified in `Cargo.toml`

### Building from Source
- You will need your own Discord API token to run your own version of this bot
- Create an application in the Discord Developer Portal
- Add the following to `Secrets.toml` in the root of the repository:
```toml
DISCORD_TOKEN = "<discord token>"
````
- Then, to build and run your local version:
```bash
cargo install cargo-shuttle
git clone https://github.com/Starkiller645/economist && cd economist
cargo shuttle run
```

- For more information on cloud deployments, please see [Shuttle's Docs](https://docs.shuttle.rs/)

