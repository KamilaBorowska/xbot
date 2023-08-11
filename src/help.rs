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
