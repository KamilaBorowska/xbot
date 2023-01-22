use crate::{Context, Error};
use once_cell::sync::Lazy;
use poise::command;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serenity::model::channel::AttachmentType;
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
    ctx: Context<'_>,
    code: &str,
    int_main: &str,
    int_main_wrapper: impl FnOnce(&str) -> String,
    runner: &str,
) -> Result<(), Error> {
    let contents = strip_code(code);
    let contents = if contents.contains(int_main) {
        contents.to_string()
    } else {
        int_main_wrapper(contents.trim())
    };
    let Response { output, status } = {
        ctx.data()
            .client
            .post(&ctx.data().sandbox_url)
            .json(&Command {
                stdin: "",
                code: runner,
                files: Files {
                    code: File { contents },
                },
            })
            .send()
            .await?
            .json()
            .await?
    };
    let output = FILTER.replace_all(&output, "").replace("\x7F\x7F", "\x7F");
    post_output(ctx, &output, status).await
}

async fn post_output(ctx: Context<'_>, output: &str, status: Option<i32>) -> Result<(), Error> {
    let formatted;
    let status_message = match status {
        Some(0) => "",
        Some(status) => {
            formatted = format!("Exited with status code {}\n", status);
            &formatted
        }
        None => "Killed the process due to timeout\n",
    };
    if output.len() > 800 || more_than_15_newlines(&output) {
        ctx.send(|m| {
            m.attachment(AttachmentType::Bytes {
                data: output.as_bytes().into(),
                filename: "output.txt".into(),
            })
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
        ctx.say(message.0).await?;
    }
    Ok(())
}

#[command(prefix_command, track_edits)]
/// Evaluate C++ code.
///
/// Evaluate C++ code. If code contains `int main` it will be interpreted \
/// as a complete program, otherwise the code will be evaluated as an \
/// expression.
///
/// Example: `!xb ceval std::string("Hello, ") + "world!"`
pub async fn ceval(ctx: Context<'_>, #[rest] code: String) -> Result<(), Error> {
    eval(
        ctx,
        &code,
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

#[command(prefix_command, track_edits)]
/// Evaluate Rust code.
///
/// Evaluate Rust code. If code contains `fn main` it will be \
/// interpreted as a complete program, otherwise the code will \
/// be evaluated as an expression.
///
/// Example: `!xb rusteval format!("Hello, {}!", "world")`
pub async fn rusteval(ctx: Context<'_>, #[rest] code: String) -> Result<(), Error> {
    eval(
        ctx,
        &code,
        "fn main",
        |rest| {
            format!("fn expr() -> impl std::fmt::Debug {{\n{}\n}} fn main() {{ println!(\"{{:#?}}\", expr()); }}", rest)
        },
        "mv code{,.rs}; $RUST_NIGHTLY/bin/rustc --edition 2021 code.rs && ./code",
    ).await
}

#[command(prefix_command, track_edits)]
/// Compiles C code and outputs 6502 assembly.
///
/// Uses Godbolt Compiler Explorer and llvm-mos internally (https://godbolt.org/).
///
/// Example: `!xb casm unsigned char add1(unsigned char v) { return v + 1; }`
pub async fn casm(ctx: Context<'_>, #[rest] code: String) -> Result<(), Error> {
    #[derive(Serialize)]
    struct Compile<'a> {
        source: &'a str,
        options: Options,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Options {
        user_arguments: &'static str,
    }
    #[derive(Deserialize)]
    struct Response {
        code: Option<i32>,
        stdout: Vec<Line>,
        stderr: Vec<Line>,
        asm: Vec<Line>,
    }
    #[derive(Deserialize)]
    struct Line {
        text: String,
    }
    let code = format!("#include <cstdint>\n{}", strip_code(&code));
    let response: Response = ctx
        .data()
        .client
        .post("https://godbolt.org/api/compiler/mos-nes-nrom-trunk/compile")
        .header("Accept", "application/json")
        .json(&Compile {
            source: &code,
            options: Options {
                user_arguments: "-Os -fno-color-diagnostics -g0 -mcpu=mosw65816 --std=c++20",
            },
        })
        .send()
        .await?
        .json()
        .await?;
    let output: String = [&response.stdout, &response.stderr, &response.asm]
        .into_iter()
        .flatten()
        .flat_map(|Line { text }| [text, "\n"])
        .collect();
    post_output(ctx, &output, response.code).await
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
