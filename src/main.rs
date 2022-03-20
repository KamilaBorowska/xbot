mod eval;
mod help;

use eval::EVAL_GROUP;
use help::HELP;
use serenity::framework::StandardFramework;
use serenity::http::Http;
use serenity::Client;
use std::env;

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
        .help(&HELP)
        .group(&EVAL_GROUP);
    let mut client = Client::builder(&token)
        .framework(framework)
        .await
        .expect("Error creating client");
    client.start().await.unwrap();
}
