// SPDX-FileCopyrightText: 2023 Konrad Borowski <konrad@borowski.pw>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::cmp;

use crate::Context;
use anyhow::Result;
use cairo::{Format, ImageSurface};
use pango::{prelude::FontMapExt, FontDescription, Layout};
use pangocairo::FontMap;
use poise::command;
use serenity::model::channel::AttachmentType;

fn create_png(text: &str) -> Result<Vec<u8>> {
    let font_map = FontMap::new();
    let pango_context = font_map.create_context();
    let layout = Layout::new(&pango_context);
    let mut font_description = FontDescription::new();
    font_description.set_family("sans-serif");
    font_description.set_absolute_size((16 * pango::SCALE).into());
    layout.set_font_description(Some(&font_description));
    // Replacement necessary to avoid GStrInteriorNulError errors
    layout.set_text(&text.replace('\0', "\u{FFFD}"));
    layout.context_changed();
    let mut extents = layout.pixel_extents().1;
    if extents.width().saturating_mul(extents.height()) > 1_000_000 {
        layout.set_text("Out of memory");
        layout.context_changed();
        extents = layout.pixel_extents().1;
    }
    let surface = ImageSurface::create(
        Format::Rgb24,
        cmp::max(extents.width(), 1),
        cmp::max(extents.height(), 1),
    )?;
    let cairo_context = cairo::Context::new(&surface)?;
    cairo_context.set_source_rgb(1.0, 1.0, 1.0);
    cairo_context.paint()?;
    cairo_context.set_source_rgb(0.0, 0.0, 0.0);
    pangocairo::show_layout(&cairo_context, &layout);
    let mut out = Vec::new();
    surface.write_to_png(&mut out)?;
    Ok(out)
}

#[command(prefix_command, track_edits, hide_in_help)]
pub async fn png(ctx: Context<'_>, #[rest] text: Option<String>) -> Result<()> {
    let out = tokio::task::spawn_blocking(move || {
        create_png(text.as_deref().unwrap_or("Unknown command."))
    })
    .await??;
    ctx.send(|m| {
        m.attachment(AttachmentType::Bytes {
            data: out.into(),
            filename: String::from("output.png"),
        })
    })
    .await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::create_png;
    use anyhow::Result;
    use png::Decoder;
    use quickcheck::quickcheck;

    quickcheck! {
        fn png_is_decodable(text: String) -> Result<()> {
            let png = create_png(&text)?;
            let decoder = Decoder::new(png.as_slice());
            let mut reader = decoder.read_info()?;
            let mut buf = vec![0; reader.output_buffer_size()];
            reader.next_frame(&mut buf)?;
            Ok(())
        }
    }
}
