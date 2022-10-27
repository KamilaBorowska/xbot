mod eval;
mod help;
mod register;

use poise::{EditTracker, Framework, FrameworkOptions, PrefixFrameworkOptions};
use reqwest::Client;
use serenity::model::gateway::GatewayIntents;
use std::env;
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    sandbox_url: String,
    client: Client,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    Framework::builder()
        .options(FrameworkOptions {
            commands: vec![
                help::help(),
                register::register(),
                eval::ceval(),
                eval::rusteval(),
            ],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some("!xb ".into()),
                edit_tracker: Some(EditTracker::for_timespan(Duration::from_secs(300))),
                ..Default::default()
            },
            ..Default::default()
        })
        .token(token)
        .intents(
            GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::MESSAGE_CONTENT,
        )
        .user_data_setup(|_ctx, _ready, _framework| {
            Box::pin(async {
                Ok(Data {
                    sandbox_url: env::var("SANDBOX_URL")?,
                    client: Client::new(),
                })
            })
        })
        .run()
        .await
        .unwrap();
}
