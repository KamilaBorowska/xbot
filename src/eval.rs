use crate::SharedKey;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serenity::client::Context;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;
use serenity::utils::MessageBuilder;

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

fn more_than_15_newlines(s: &str) -> bool {
    s.bytes().filter(|&c| c == b'\n').nth(15 - 1).is_some()
}

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
    let Response { stdout, stderr } = {
        let shared = ctx.data.read().await;
        let shared = shared.get::<SharedKey>().unwrap();
        shared
            .client
            .post(&shared.sandbox_url)
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
            .await?
    };
    let stderr = NIX_STORE.replace_all(&stderr, "");
    if stdout.len() > 800
        || stderr.len() > 800
        || more_than_15_newlines(&stdout)
        || more_than_15_newlines(&stderr)
    {
        msg.channel_id
            .send_message(ctx, |m| {
                if !stdout.is_empty() {
                    m.add_file((stdout.as_bytes(), "stdout.txt"));
                }
                if !stderr.is_empty() {
                    m.add_file((stderr.as_bytes(), "stderr.txt"));
                }
                m.reference_message(msg)
                    .allowed_mentions(|f| f.replied_user(false))
            })
            .await?;
    } else {
        let mut output = MessageBuilder::new();
        if !stdout.is_empty() {
            output.push_codeblock_safe(&stdout, None);
        }
        if !stderr.is_empty() {
            if !stdout.is_empty() {
                output.push_line("");
            }
            output.push_line("Error output:");
            output.push_codeblock_safe(&stderr, None);
        }
        if output.0.is_empty() {
            output.push_italic("(no output)");
        }
        msg.reply(&ctx, output.0).await?;
    }
    Ok(())
}

#[command]
#[description = "Evaluate C++ code. If code contains `int main` it will be interpreted as a complete program, otherwise the code will be evaluated as an expression."]
#[example = r#"std::string("Hello, ") + "world!""#]
#[bucket = "eval"]
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
                if contains_return { "" } else { "return ({" },
                rest,
                if rest.ends_with(';') || rest.ends_with('}') {
                    ""
                } else {
                    ";"
                },
                if contains_return { "" } else { "});" },
            )
        },
        "mv code{,.cpp}; clang++ -std=c++17 -Wall -Wextra code.cpp && ./a.out",
    )
    .await
}

#[command]
#[description = "Evaluate Rust code. If code contains `fn main` it will be interpreted as a complete program, otherwise the code will be evaluated as an expression."]
#[example = r#"format!("Hello, {}!", "world")"#]
#[bucket = "eval"]
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

#[group("Code evaluation commands")]
#[commands(ceval, rusteval)]
struct Eval;

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
