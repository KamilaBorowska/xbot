// SPDX-FileCopyrightText: 2022 - 2023 Konrad Borowski <konrad@borowski.pw>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::Context;
use anyhow::Result;
use once_cell::sync::Lazy;
use poise::command;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serenity::model::channel::AttachmentType;
use serenity::utils::MessageBuilder;

#[derive(Serialize)]
struct Command<'a, F> {
    stdin: &'a str,
    code: &'a str,
    files: F,
}

#[derive(Serialize)]
struct Files {
    code: File,
}

#[derive(Serialize)]
struct NoFiles {}

#[derive(Serialize)]
struct File {
    contents: String,
}

#[derive(Deserialize)]
struct Response {
    output: String,
    status: Option<i32>,
}

#[derive(Debug, PartialEq, Eq)]
struct Parsed<'a> {
    options: &'a str,
    code: &'a str,
}

impl<'a> Parsed<'a> {
    fn new(options: &'a str, code: &'a str) -> Parsed<'a> {
        Self {
            options: options.trim(),
            code,
        }
    }
}

fn parse_code(mut s: &str) -> Parsed<'_> {
    if let Some((options, without_prefix)) = s.split_once("```") {
        if let Some((first_line, rest)) = without_prefix.split_once('\n') {
            if first_line
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'+')
            {
                if let Some(code) = rest.strip_suffix("```") {
                    return Parsed::new(options, code);
                }
            }
        }
    }
    let mut options = "";
    if let Some((o, rest)) = s.split_once('`') {
        options = o;
        if let Some(code) = rest.strip_suffix('`') {
            s = code;
            while let Some(code) = s.strip_prefix('`').and_then(|s| s.strip_suffix('`')) {
                s = code;
            }
        }
    }
    Parsed::new(options, s)
}

static FILTER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"/nix/store/[^/]+-gcc-[^/]+/include/c[+][+]/[^/]+/|\x7F[EO]").unwrap()
});

fn more_than_15_newlines(s: &str) -> bool {
    s.bytes().filter(|&c| c == b'\n').nth(15 - 1).is_some()
}

async fn sandbox_request<F>(ctx: Context<'_>, command: &Command<'_, F>) -> Result<Response>
where
    F: Serialize,
{
    Ok(ctx
        .data()
        .client
        .post(&ctx.data().sandbox_url)
        .json(command)
        .send()
        .await?
        .json()
        .await?)
}

async fn eval(
    ctx: Context<'_>,
    code: &str,
    int_main: &str,
    int_main_wrapper: impl FnOnce(&str) -> String,
    runner: impl FnOnce(&str) -> String,
) -> Result<()> {
    let Parsed { options, code } = parse_code(code);
    let code = if code.contains(int_main) {
        code.to_string()
    } else {
        int_main_wrapper(code.trim())
    };
    let Response { output, status } = sandbox_request(
        ctx,
        &Command {
            stdin: "",
            code: &runner(options),
            files: Files {
                code: File { contents: code },
            },
        },
    )
    .await?;
    let output = FILTER.replace_all(&output, "").replace("\x7F\x7F", "\x7F");
    post_output(ctx, &output, status).await
}

async fn post_output(ctx: Context<'_>, output: &str, status: Option<i32>) -> Result<()> {
    let formatted;
    let status_message = match status {
        Some(0) => "",
        Some(status) => {
            formatted = format!("Exited with status code {status}\n");
            &formatted
        }
        None => "Killed the process due to timeout\n",
    };
    if output.len() > 800 || more_than_15_newlines(output) {
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
            message.push_codeblock_safe(output, None);
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
pub async fn ceval(ctx: Context<'_>, #[rest] code: String) -> Result<()> {
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
        |opt| {
            format!("mv code{{,.cpp}}; clang++ -std=c++17 -Wall -Wextra {opt} code.cpp && ./a.out")
        },
    )
    .await
}

static FEATURES: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?:\s*#\s*!\s*\[\s*feature\s*\([^)]+\)\s*\])*").unwrap());

#[command(prefix_command, track_edits)]
/// Evaluate Rust code.
///
/// Evaluate Rust code. If code contains `fn main` it will be \
/// interpreted as a complete program, otherwise the code will \
/// be evaluated as an expression.
///
/// Example: `!xb rusteval format!("Hello, {}!", "world")`
pub async fn rusteval(ctx: Context<'_>, #[rest] code: String) -> Result<()> {
    eval(
        ctx,
        &code,
        "fn main",
        |rest| {
            let regex_match = FEATURES.find(rest).unwrap();
            let features = regex_match.as_str();
            let inner = &rest[regex_match.end()..];
            format!(
                concat!(
                    "{features}\n",
                    "fn expr() -> impl std::fmt::Debug + 'static {{\n",
                    "{inner}\n",
                    "}}\n",
                    "fn main() {{\n",
                    "    fn is_unit<T: 'static>(_: &T) -> bool {{\n",
                    "        std::any::TypeId::of::<()>() == std::any::TypeId::of::<T>()\n",
                    "    }}\n",
                    "    let v = expr();\n",
                    "    if !is_unit(&v) {{\n",
                    "        println!(\"{{v:#?}}\");\n",
                    "    }}\n",
                    "}}\n",
                ),
                features = features,
                inner = inner,
            )
        },
        |opt| {
            format!(
                "mv code{{,.rs}}; $RUST_NIGHTLY/bin/rustc --edition 2021 {opt} code.rs && ./code"
            )
        },
    )
    .await
}

const PYTHON_EVALUATOR: &str = r#"
import ast
code = open("code").read()
a = ast.parse(code, "code")
last_expression = None
if a.body and isinstance(a.body[-1], ast.Expr):
    last_expression = a.body.pop()
g = {}
l = {}
exec(compile(a, "code", "exec"), g, l)
if last_expression:
    eval(compile(ast.Interactive([last_expression]), "code", "single"), g, l)
"#;

#[command(prefix_command, track_edits)]
/// Evaluate Python code.
///
/// Example: `!xb pyeval print(2 + 2)`
pub async fn pyeval(ctx: Context<'_>, #[rest] code: String) -> Result<()> {
    eval(
        ctx,
        &code,
        "",
        |_| unreachable!(),
        |opt| format!("python3 {opt} -u -c '{PYTHON_EVALUATOR}'"),
    )
    .await
}

#[command(prefix_command, slash_command, track_edits)]
/// Fix mojibake.
///
/// Example: `!xb ftfy âœ”`
pub async fn ftfy(ctx: Context<'_>, #[rest] text: String) -> Result<()> {
    let Response { output, .. } = sandbox_request(
        ctx,
        &Command {
            stdin: &text,
            code: "ftfy",
            files: NoFiles {},
        },
    )
    .await?;
    ctx.say(output).await?;
    Ok(())
}

#[command(prefix_command, track_edits)]
/// Compiles C code and outputs 6502 assembly.
///
/// Uses Godbolt Compiler Explorer and llvm-mos internally (https://godbolt.org/).
///
/// Example: `!xb casm unsigned char add1(unsigned char v) { return v + 1; }`
pub async fn casm(ctx: Context<'_>, #[rest] code: String) -> Result<()> {
    #[derive(Serialize)]
    struct Compile<'a> {
        source: &'a str,
        options: Options,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Options {
        user_arguments: String,
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
    let Parsed { options, code } = parse_code(&code);
    let code = format!("#include <cstdint>\n{code}");
    let user_arguments =
        format!("-Os -fno-color-diagnostics -g0 -mcpu=mosw65816 --std=c++20 {options}");
    let response: Response = ctx
        .data()
        .client
        .post("https://godbolt.org/api/compiler/mos-nes-nrom-trunk/compile")
        .header("Accept", "application/json")
        .json(&Compile {
            source: &code,
            options: Options { user_arguments },
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
    use super::{parse_code, Parsed};

    #[test]
    fn strip_code() {
        assert_eq!(
            parse_code("test"),
            Parsed {
                options: "",
                code: "test",
            },
        );
        assert_eq!(
            parse_code("`code`"),
            Parsed {
                options: "",
                code: "code",
            },
        );
        assert_eq!(
            parse_code("``foo``"),
            Parsed {
                options: "",
                code: "foo",
            },
        );
        assert_eq!(
            parse_code("```\nbar\n```"),
            Parsed {
                options: "",
                code: "bar\n",
            },
        );
        assert_eq!(
            parse_code("```example code here\n```"),
            Parsed {
                options: "",
                code: "example code here\n",
            },
        );
        assert_eq!(
            parse_code("```c++\nexample\n```"),
            Parsed {
                options: "",
                code: "example\n",
            },
        );
        assert_eq!(
            parse_code("-Wall ```c++\nhi\n```"),
            Parsed {
                options: "-Wall",
                code: "hi\n",
            },
        );
        assert_eq!(
            parse_code("-Wall `hi`"),
            Parsed {
                options: "-Wall",
                code: "hi",
            },
        );
        assert_eq!(
            parse_code("-Wall\n```c++\nhi\n```"),
            Parsed {
                options: "-Wall",
                code: "hi\n",
            },
        );
    }
}
