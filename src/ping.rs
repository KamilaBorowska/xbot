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
