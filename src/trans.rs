// xbot - Discord bot
// Copyright (C) 2022-2023  Konrad Borowski
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::{Context, Data};
use anyhow::{Error, Result};
use poise::{command, Command};
use serde::{Deserialize, Serialize};

pub(crate) fn trans_merged() -> Command<Data, Error> {
    Command {
        prefix_action: trans_prefix().prefix_action,
        ..trans()
    }
}

const SOURCE_LANGUAGES: &[&str] = &[
    "BG", "CS", "DA", "DE", "EL", "EN", "ES", "ET", "FI", "FR", "HU", "ID", "IT", "JA", "LT", "LV",
    "NL", "PL", "PT", "RO", "SK", "SL", "SV", "TR", "UK", "ZH",
];
const TARGET_LANGUAGES: &[&str] = &[
    "BG", "CS", "DA", "DE", "EL", "EN", "EN-GB", "EN-US", "ES", "ET", "FI", "FR", "HU", "ID", "IT",
    "JA", "LT", "LV", "NL", "PL", "PT", "PT-BR", "PT-PT", "RO", "RU", "SK", "SL", "SV", "TR", "UK",
    "ZH",
];

#[command(prefix_command)]
async fn trans_prefix(ctx: Context<'_>, #[rest] text: Option<String>) -> Result<()> {
    let Some(text) = text else {
        // Trans flag
        ctx.say("\u{1F3F3}\u{FE0F}\u{200D}\u{26A7}\u{FE0F}").await?;
        return Ok(());
    };
    let mut text: &str = &text;
    let mut from = None;
    let mut to = None;
    if let Some((a, b)) = text.trim_start().split_once(char::is_whitespace) {
        if let Some((source, target)) = a.split_once('-') {
            from = (!source.is_empty()).then_some(source);
            to = (!target.is_empty()).then_some(target);
            text = b;
        }
    }
    run_translation(ctx, from, to, text).await
}

/// Translate text using DeepL.
///
/// Translate text using DeepL. An optional source or target language can be provided. \
/// When source is not provided, DeepL will try to guess the language, when target is \
/// not provided, it will be assumed to be English.
///
/// Examples:
/// `!xb trans -fr Hello, world!`
/// `!xb trans pl- Witaj świecie.`
/// `!xb trans et-cs Tere, maailm!`
/// `!xb trans Ciao mondo!`
/// `/trans こんにちは世界！`
#[command(prefix_command, track_edits, slash_command)]
async fn trans(
    ctx: Context<'_>,
    #[description = "Source language"]
    #[autocomplete = "source_language"]
    from: Option<String>,
    #[description = "Target language"]
    #[autocomplete = "target_language"]
    to: Option<String>,
    #[description = "Text to translate"]
    #[rest]
    text: String,
) -> Result<()> {
    run_translation(ctx, from.as_deref(), to.as_deref(), &text).await
}

async fn source_language<'a>(
    _ctx: Context<'a>,
    partial: &'a str,
) -> impl Iterator<Item = String> + 'a {
    autocomplete_case_insensitive(SOURCE_LANGUAGES, partial)
}

async fn target_language<'a>(
    _ctx: Context<'a>,
    partial: &'a str,
) -> impl Iterator<Item = String> + 'a {
    autocomplete_case_insensitive(TARGET_LANGUAGES, partial)
}

fn autocomplete_case_insensitive<'a>(
    list: &'a [&str],
    partial: &'a str,
) -> impl Iterator<Item = String> + 'a {
    list.iter()
        .filter(|elem| {
            elem.get(..partial.len()).map_or(false, |trimmed_elem| {
                trimmed_elem.eq_ignore_ascii_case(partial)
            })
        })
        .map(|elem| String::from(*elem))
}

#[derive(Serialize)]
struct TranslateRequest<'a> {
    text: &'a str,
    source_lang: Option<&'a str>,
    target_lang: &'a str,
}

#[derive(Deserialize)]
struct TranslateResponse {
    translations: [Translation; 1],
}

#[derive(Deserialize)]
struct Translation {
    text: String,
}

async fn run_translation(
    ctx: Context<'_>,
    from: Option<&str>,
    to: Option<&str>,
    text: &str,
) -> Result<()> {
    let api_key = &ctx.data().deepl_auth_key;
    let source_lang = match from {
        Some(lang) => {
            let lang = lang.to_uppercase();
            if !SOURCE_LANGUAGES.contains(&lang.as_str()) {
                ctx.say(format!(
                    concat!(
                        "Unrecognized source language {lang}, ",
                        "please refer to list of supported languages at ",
                        "https://www.deepl.com/docs-api/translate-text/."
                    ),
                    lang = lang,
                ))
                .await?;
                return Ok(());
            }
            Some(lang)
        }
        None => None,
    };
    let uppercase_target;
    let target_lang = match to {
        None => "EN-US",
        Some(to) => {
            uppercase_target = to.to_uppercase();
            if !TARGET_LANGUAGES.contains(&uppercase_target.as_str()) {
                ctx.say(format!(
                    concat!(
                        "Unrecognized target language {lang}, ",
                        "please refer to list of supported languages at ",
                        "https://www.deepl.com/docs-api/translate-text/."
                    ),
                    lang = uppercase_target,
                ))
                .await?;
                return Ok(());
            }
            &uppercase_target
        }
    };
    let response: TranslateResponse = ctx
        .data()
        .client
        .post("https://api-free.deepl.com/v2/translate")
        .header("Authorization", format!("DeepL-Auth-Key {api_key}"))
        .form(&TranslateRequest {
            text,
            source_lang: source_lang.as_deref(),
            target_lang,
        })
        .send()
        .await?
        .json()
        .await?;
    ctx.say(&response.translations[0].text).await?;
    Ok(())
}
