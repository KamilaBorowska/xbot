mod eval;
mod help;
mod register;
mod trans;

use log::error;
use poise::{EditTracker, Framework, FrameworkError, FrameworkOptions, PrefixFrameworkOptions};
use reqwest::Client;
use serenity::model::gateway::GatewayIntents;
use std::env;
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    sandbox_url: String,
    deepl_auth_key: String,
    client: Client,
}

async fn on_error(error: FrameworkError<'_, Data, Error>) {
    let result = match error {
        FrameworkError::UnknownCommand { ctx, msg, .. } => msg
            .channel_id
            .say(ctx, "Unknown command.")
            .await
            .map(|_| ()),
        _ => poise::builtins::on_error(error).await,
    };
    if let Err(e) = result {
        error!("Error while handling error {e}");
    }
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
                eval::pyeval(),
                eval::ftfy(),
                eval::casm(),
                trans::trans_merged(),
            ],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some("!xb ".into()),
                edit_tracker: Some(EditTracker::for_timespan(Duration::from_secs(300))),
                ..Default::default()
            },
            on_error: |e| Box::pin(on_error(e)),
            ..Default::default()
        })
        .token(token)
        .intents(
            GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::MESSAGE_CONTENT,
        )
        .setup(|_ctx, _ready, _framework| {
            Box::pin(async {
                Ok(Data {
                    sandbox_url: env::var("SANDBOX_URL")?,
                    deepl_auth_key: env::var("DEEPL_AUTH_KEY")?,
                    client: Client::new(),
                })
            })
        })
        .run()
        .await
        .unwrap();
}
