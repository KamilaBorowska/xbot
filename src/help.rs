// SPDX-FileCopyrightText: 2022 - 2023 Konrad Borowski <konrad@borowski.pw>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::Context;
use anyhow::Result;
use poise::{builtins::HelpConfiguration, command};

/// Shows this menu.
#[command(prefix_command, slash_command, track_edits)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "autocomplete_command"]
    command: Option<String>,
) -> Result<()> {
    poise::builtins::help(ctx, command.as_deref(), HelpConfiguration::default()).await?;
    Ok(())
}

async fn autocomplete_command<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> impl Iterator<Item = String> + 'a {
    ctx.framework()
        .options()
        .commands
        .iter()
        .filter(move |cmd| !cmd.hide_in_help && cmd.name.starts_with(partial))
        .map(|cmd| cmd.name.to_string())
}
