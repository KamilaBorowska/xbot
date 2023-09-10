// SPDX-FileCopyrightText: 2023 Konrad Borowski <konrad@borowski.pw>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::Context;
use anyhow::Result;
use poise::command;
use std::time::Instant;

/// Get the link to the source code for this bot.
#[command(prefix_command, hide_in_help)]
pub async fn ping(ctx: Context<'_>) -> Result<()> {
    let now = Instant::now();
    ctx.say("Pong!")
        .await?
        .edit(ctx, |b| {
            b.content(format!("Pong! Took {:?}.", now.elapsed()))
        })
        .await?;
    Ok(())
}
