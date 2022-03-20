mod eval;
mod help;

use eval::EVAL_GROUP;
use help::HELP;
use reqwest::Client as ReqwestClient;
use serenity::framework::StandardFramework;
use serenity::http::Http;
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

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let http = Http::new_with_token(&token);
    let bot_id = http
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
        .bucket("eval", |b| b.delay(2).time_span(30).limit(8))
        .await
        .help(&HELP)
        .group(&EVAL_GROUP);
    let mut client = Client::builder(&token)
        .framework(framework)
        .await
        .expect("Error creating client");
    {
        let mut data = client.data.write().await;
        data.insert::<SharedKey>(Shared {
            sandbox_url: env::var("SANDBOX_URL").expect("Sandbox URL"),
            client: ReqwestClient::new(),
        });
    }
    client.start().await.unwrap();
}
