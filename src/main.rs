use serenity::prelude::*;
use serenity::model::{
    channel::Message,
    gateway::Ready
};
use serenity::utils::MessageBuilder;
use serenity::async_trait;
use serenity::model::application::command::Command;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::id::GuildId;
use std::env;

pub mod commands;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, cx: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(e) = msg.channel_id.say(&cx.http, "pong!").await {
                eprintln!("Message: Error: {:?}", e);
            }
        }
    }

    async fn interaction_create(&self, cx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(cmd) = interaction {
            println!("Received command interaction: {:#?}", cmd);

            let content = match cmd.data.name.as_str() {
                "version" => commands::version::run(&cmd),
                _ => "Not implemented yet :(".to_string()
            };

            if let Err(why) = cmd
                .create_interaction_response(&cx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content(content))
                })
                .await
            {
                println!("Cannot respond to slash command: {}", why);
            }
        }
    }

    async fn ready(&self, cx: Context, ready: Ready) {
        println!("Bot `{}` is up and running!", ready.user.name);

        let commands = Command::create_global_application_command(&cx.http, |command| {
            commands::ping::register(command)
        })
        .await;

        println!("The following guild commands are enabled: {:?}", commands);
    
    }
}

#[tokio::main]
async fn main() {   
    dotenvy::dotenv().expect("Error: Failed reading environment variables");

    let discord_token = env::var("DISCORD_TOKEN").expect("Error: DISCORD_TOKEN environment variable not set!");
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::DIRECT_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&discord_token, intents).event_handler(Handler).await.expect("Error: could not create client");
    
    if let Err(e) = client.start().await {
        eprintln!("Client: Error: {:?}", e);
    }
}


