mod eval;

use eval::EVAL_GROUP;
use serenity::client::Context;
use serenity::framework::standard::macros::help;
use serenity::framework::standard::{
    help_commands, Args, CommandGroup, CommandResult, HelpOptions,
};
use serenity::framework::StandardFramework;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::id::UserId;
use serenity::Client;
use std::collections::HashSet;
use std::env;

#[help]
#[strikethrough_commands_tip_in_dm = ""]
#[strikethrough_commands_tip_in_guild = ""]
#[available_text = ""]
#[max_levenshtein_distance(3)]
async fn help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
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
        .help(&HELP)
        .group(&EVAL_GROUP);
    let mut client = Client::builder(&token)
        .framework(framework)
        .await
        .expect("Error creating client");
    client.start().await.unwrap();
}
