use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use serenity::client::Context;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{Args, CommandResult};
use serenity::framework::StandardFramework;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::Client;
use std::env;

#[derive(Serialize)]
struct Command<'a> {
    stdin: &'static str,
    code: &'a str,
    files: Files,
}

#[derive(Serialize)]
struct Files {
    code: File,
}

#[derive(Serialize)]
struct File {
    contents: String,
}

#[derive(Deserialize)]
struct Response {
    stdout: String,
    stderr: String,
}

fn strip_code(mut s: &str) -> &str {
    if let Some((first_line, rest)) = s.split_once('\n') {
        if let Some(without_prefix) = first_line.strip_prefix("```") {
            if without_prefix
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'+')
            {
                if let Some(rest) = rest.strip_suffix("```") {
                    return rest;
                }
            }
        }
    }
    while let Some(code) = s.strip_prefix('`').and_then(|s| s.strip_suffix('`')) {
        s = code;
    }
    s
}

static NIX_STORE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"/nix/store/[^/]+-gcc-[^/]+/include/c[+][+]/[^/]+/").unwrap());

static SANDBOX_URL: Lazy<String> = Lazy::new(|| env::var("SANDBOX_URL").unwrap());

async fn eval(
    ctx: &Context,
    msg: &Message,
    args: Args,
    int_main: &str,
    int_main_wrapper: impl FnOnce(&str) -> String,
    code: &str,
) -> CommandResult {
    let contents = strip_code(args.rest());
    let contents = if contents.contains(int_main) {
        contents.to_string()
    } else {
        int_main_wrapper(contents.trim())
    };
    let Response { stdout, stderr } = ReqwestClient::new()
        .post(&*SANDBOX_URL)
        .json(&Command {
            stdin: "",
            code,
            files: Files {
                code: File { contents },
            },
        })
        .send()
        .await?
        .json()
        .await?;
    let mut output = String::new();
    if !stdout.is_empty() {
        output.push_str("```\n");
        let stdout_trimmed: String = stdout.chars().take(800).collect();
        output.push_str(&stdout_trimmed);
        if stdout_trimmed != stdout {
            output.push_str("\n[trimmed]");
        }
        output.push_str("\n```");
    }
    if !stderr.is_empty() {
        if !stdout.is_empty() {
            output.push('\n');
        }
        output.push_str("Error output:\n```\n");
        let stderr_trimmed: String = NIX_STORE
            .replace_all(&stderr, "")
            .chars()
            .take(800)
            .collect();
        output.push_str(&stderr_trimmed);
        if stderr_trimmed != stderr {
            output.push_str("\n[trimmed]");
        }
        output.push_str("\n```");
    }
    if output.is_empty() {
        output.push_str("_(no output)_")
    }
    msg.reply(&ctx, output).await?;
    Ok(())
}

#[command]
async fn ceval(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    eval(
        ctx,
        msg,
        args,
        "int main",
        |rest| {
            let contains_return = rest.contains("return");
            format!(
                concat!(
                    "#include <cstdio>\n",
                    "#include <iostream>\n",
                    "#include <string>\n",
                    "#include <string_view>\n",
                    "auto expr() {{ \n{}{}{}{}\n }} int main() {{ std::cout << expr(); }}"
                ),
                if contains_return {
                    ""
                } else {
                    "return ({"
                },
                rest,
                if rest.ends_with(';') || rest.ends_with('}') {
                    ""
                } else {
                    ";"
                },
                if contains_return {
                    ""
                } else {
                    "});"
                },
            )
        },
        "mv code{,.cpp}; clang++ -std=c++17 -Wall -Wextra code.cpp && ./a.out",
    ).await
}

#[command]
async fn rusteval(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    eval(
        ctx,
        msg,
        args,
        "fn main",
        |rest| {
            format!("fn expr() -> impl std::fmt::Debug {{\n{}\n}} fn main() {{ println!(\"{{:#?}}\", expr()); }}", rest)
        },
        "mv code{,.rs}; $RUST_NIGHTLY/bin/rustc code.rs && ./code",
    ).await
}

#[group("eval")]
#[commands(ceval, rusteval)]
struct Eval;

#[tokio::main]
async fn main() {
    env_logger::init();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let http = Http::new_with_token(&token);
    let bot_id = http
        .get_current_user()
        .await
        .expect("Application information to be obtainable")
        .id;
    let framework = StandardFramework::new()
        .configure(|c| c.on_mention(Some(bot_id)).prefix("!xb "))
        .group(&EVAL_GROUP);
    let mut client = Client::builder(&token)
        .framework(framework)
        .await
        .expect("Error creating client");
    client.start().await.unwrap();
}

#[cfg(test)]
mod test {
    #[test]
    fn strip_code() {
        use super::strip_code;
        assert_eq!(strip_code("test"), "test");
        assert_eq!(strip_code("`code`"), "code");
        assert_eq!(strip_code("``foo``"), "foo");
        assert_eq!(strip_code("```\nbar\n```"), "bar\n");
        assert_eq!(
            strip_code("```example code here\n```"),
            "example code here\n"
        );
        assert_eq!(strip_code("```c++\nexample\n```"), "example\n");
    }
}
