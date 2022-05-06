mod eval;
mod help;

use eval::EVAL_GROUP;
use help::HELP;
use log::error;
use reqwest::Client as ReqwestClient;
use serenity::client::Context;
use serenity::framework::standard::macros::hook;
use serenity::framework::standard::DispatchError;
use serenity::framework::StandardFramework;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::gateway::GatewayIntents;
use serenity::prelude::TypeMapKey;
use serenity::Client;
use std::env;

struct SharedKey;

impl TypeMapKey for SharedKey {
    type Value = Shared;
}

struct Shared {
    sandbox_url: String,
    client: ReqwestClient,
}

#[hook]
async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError, _command_name: &str) {
    if let DispatchError::Ratelimited(info) = error {
        if info.is_first_try {
            let message = format!("Try this again in {}s", info.as_secs() + 1);
            if let Err(e) = msg.channel_id.say(&ctx, &message).await {
                error!("Error while warning the user about rate limit: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let bot_id = Http::new(&token)
        .get_current_user()
        .await
        .expect("Application information to be obtainable")
        .id;
    let framework = StandardFramework::new()
        .configure(|c| {
            c.on_mention(Some(bot_id))
                .prefix("!xb ")
                .no_dm_prefix(true)
                .case_insensitivity(true)
        })
        .on_dispatch_error(dispatch_error)
        .bucket("eval", |b| b.time_span(60).limit(8))
        .await
        .help(&HELP)
        .group(&EVAL_GROUP);
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .await
        .expect("Error creating client");
    client.data.write().await.insert::<SharedKey>(Shared {
        sandbox_url: env::var("SANDBOX_URL").expect("Sandbox URL"),
        client: ReqwestClient::new(),
    });
    client.start().await.unwrap();
}
