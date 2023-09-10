// SPDX-FileCopyrightText: 2023 Konrad Borowski <konrad@borowski.pw>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::Context;
use anyhow::Result;
use poise::command;

/// Get the link to the source code for this bot.
#[command(prefix_command, slash_command, owners_only)]
pub async fn source(ctx: Context<'_>) -> Result<()> {
    ctx.say("<https://github.com/xfix/xbot>").await?;
    Ok(())
}
