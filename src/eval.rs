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
    output: String,
    status: Option<i32>,
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

static FILTER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"/nix/store/[^/]+-gcc-[^/]+/include/c[+][+]/[^/]+/|\x7F[EO]").unwrap()
});

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
    let Response { output, status } = {
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
    let formatted;
    let status_message = match status {
        Some(0) => "",
        Some(status) => {
            formatted = format!("Exited with status code {}\n", status);
            &formatted
        }
        None => "Killed the process due to timeout\n",
    };
    let output = FILTER.replace_all(&output, "").replace("\x7F\x7F", "\x7F");
    if output.len() > 800 || more_than_15_newlines(&output) {
        msg.channel_id
            .send_message(ctx, |m| {
                m.add_file((output.as_bytes(), "output.txt"));
                m.reference_message(msg)
                    .allowed_mentions(|f| f.replied_user(false))
                    .content(status_message)
            })
            .await?;
    } else {
        let mut message = MessageBuilder::new();
        message.push(status_message);
        if output.is_empty() {
            message.push_italic("(no output)");
        } else {
            message.push_codeblock_safe(&output, None);
        }
        msg.reply(&ctx, message.0).await?;
    }
    Ok(())
}

#[command]
#[example = r#"std::string("Hello, ") + "world!""#]
#[bucket = "eval"]
/// Evaluate C++ code. If code contains `int main` it will be interpreted \$
/// as a complete program, otherwise the code will be evaluated as an \$
/// expression.
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
                    "#include <vector>\n",
                    "auto expr() {{ \n{}{}{}{}\n }} ",
                    "int main() {{ std::cout << expr(); }}",
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
#[example = r#"format!("Hello, {}!", "world")"#]
#[bucket = "eval"]
/// Evaluate Rust code. If code contains `fn main` it will be \$
/// interpreted as a complete program, otherwise the code will \$
/// be evaluated as an expression.
async fn rusteval(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    eval(
        ctx,
        msg,
        args,
        "fn main",
        |rest| {
            format!("fn expr() -> impl std::fmt::Debug {{\n{}\n}} fn main() {{ println!(\"{{:#?}}\", expr()); }}", rest)
        },
        "mv code{,.rs}; $RUST_NIGHTLY/bin/rustc --edition 2021 code.rs && ./code",
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
