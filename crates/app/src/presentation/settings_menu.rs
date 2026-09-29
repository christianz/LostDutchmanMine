//! The settings menu: drawn at the window's physical resolution over a live
//! preview, with text from the pre-baked font, and hit-tested the same way.

use anyhow::{Result, anyhow};
use desktop::font::{self, BAKED_SIZE};
use desktop::menu::{self, MenuItem};
use desktop::pixels::{Pixels, picture_rect};
use desktop::settings::DisplaySettings;
use sdl2::rect::{FRect, Rect};

use super::{
    BACKGROUND, BLACK, GOLD, INK, LAYOUT, LINE, MUTED, ON_GOLD, PANEL, Presentation, ROW_HEIGHT,
    ROW_PITCH, ROW_TOP, WARNING, colour,
};

impl Presentation<'_> {
    fn layout(&mut self) {
        let (width, height) = self.output_size();
        let scale = (width as f32 / LAYOUT.0).min(height as f32 / LAYOUT.1);
        self.ui = (
            scale,
            (width as f32 - LAYOUT.0 * scale) / 2.0,
            (height as f32 - LAYOUT.1 * scale) / 2.0,
        );
    }

    fn ui_rect(&self, x: f32, y: f32, width: f32, height: f32) -> FRect {
        let (scale, left, top) = self.ui;
        FRect::new(left + x * scale, top + y * scale, width * scale, height * scale)
    }

    fn fill(&mut self, x: f32, y: f32, width: f32, height: f32, c: u32) -> Result<()> {
        let rect = self.ui_rect(x, y, width, height);
        self.canvas.set_draw_color(colour(c));
        self.canvas.fill_frect(rect).map_err(|e| anyhow!(e))
    }

    fn outline(&mut self, x: f32, y: f32, width: f32, height: f32, c: u32) -> Result<()> {
        let rect = self.ui_rect(x, y, width, height);
        self.canvas.set_draw_color(colour(c));
        self.canvas.draw_frect(rect).map_err(|e| anyhow!(e))
    }

    fn text(
        &mut self,
        (x, y): (f32, f32),
        size: f32,
        text: &str,
        c: u32,
        centre: bool,
    ) -> Result<()> {
        let scale = size / BAKED_SIZE;
        let mut x = if centre { x - font::text_width(text, size) / 2.0 } else { x };
        let [alpha, red, green, blue] = c.to_be_bytes();
        self.font.set_color_mod(red, green, blue);
        self.font.set_alpha_mod(alpha);
        for glyph in text.bytes().filter_map(font::glyph) {
            let source = Rect::new(glyph.x, glyph.y, glyph.w as u32, glyph.h as u32);
            let destination = self.ui_rect(
                x + glyph.left as f32 * scale,
                y + glyph.top as f32 * scale,
                glyph.w as f32 * scale,
                glyph.h as f32 * scale,
            );
            self.canvas.copy_f(&self.font, source, destination).map_err(|e| anyhow!(e))?;
            x += glyph.advance * scale;
        }
        Ok(())
    }

    /// Draws the settings menu over a live preview of `frame` in `draft`.
    ///
    /// # Errors
    ///
    /// When the renderer fails.
    pub fn menu(
        &mut self,
        frame: &Pixels,
        draft: DisplaySettings,
        startup: bool,
        selected: MenuItem,
        message: Option<&str>,
    ) -> Result<()> {
        self.layout();
        self.canvas.set_draw_color(colour(BACKGROUND));
        self.canvas.clear();
        if let Some(icon) = &self.icon {
            let rect = self.ui_rect(38.0, 32.0, 64.0, 64.0);
            self.canvas.copy_f(icon, None, rect).map_err(|e| anyhow!(e))?;
        }
        self.text((118.0, 64.0), 32.0, "Lost Dutchman Mine", INK, false)?;
        self.text((119.0, 91.0), 15.0, "DISPLAY & GAMEPLAY", GOLD, false)?;
        self.fill(40.0, 122.0, 960.0, 1.0, LINE)?;
        for (i, row) in (0u8..).zip(MenuItem::ROWS) {
            let y = ROW_TOP + f32::from(i) * ROW_PITCH;
            let chosen = row == selected;
            self.fill(40.0, y, 496.0, ROW_HEIGHT, PANEL)?;
            self.outline(40.0, y, 496.0, ROW_HEIGHT, if chosen { GOLD } else { LINE })?;
            self.text(
                (55.0, y + 30.0),
                18.0,
                menu::label(row, startup),
                if chosen { INK } else { MUTED },
                false,
            )?;
            self.text((270.0, y + 31.0), 22.0, "<", GOLD, false)?;
            self.text((502.0, y + 31.0), 22.0, ">", GOLD, false)?;
            self.text((388.0, y + 30.0), 17.0, &menu::value(&draft, row), INK, true)?;
        }
        self.text((568.0, 153.0), 15.0, "LIVE PREVIEW", GOLD, false)?;
        self.fill(568.0, 174.0, 432.0, 324.0, BLACK)?;
        self.upload(frame, draft)?;
        let preview = picture_rect(432, 324, menu::picture_percent(&draft));
        let area = self.ui_rect(
            568.0 + preview.x as f32,
            174.0 + preview.y as f32,
            preview.w as f32,
            preview.h as f32,
        );
        let destination = Rect::new(
            area.x().round() as i32,
            area.y().round() as i32,
            area.width().round() as u32,
            area.height().round() as u32,
        );
        if destination.width() > 0 && destination.height() > 0 {
            self.picture(destination, draft)?;
        }
        self.outline(568.0, 174.0, 432.0, 324.0, LINE)?;
        let [first, second] = menu::help(selected, startup);
        self.text((568.0, 529.0), 17.0, first, MUTED, false)?;
        self.text((568.0, 554.0), 17.0, second, MUTED, false)?;
        self.button(MenuItem::Comfort, (40.0, 566.0, 240.0), selected, startup, false)?;
        self.button(MenuItem::Original, (296.0, 566.0, 240.0), selected, startup, false)?;
        let mode = self.video.desktop_display_mode(self.display());
        let monitor = mode.map_or_else(
            |_| String::from("Monitor: unknown"),
            |mode| format!("Monitor: {} x {} / {} Hz", mode.w, mode.h, self.refresh_rate()),
        );
        self.text((568.0, 598.0), 16.0, &monitor, MUTED, false)?;
        self.fill(40.0, 639.0, 960.0, 1.0, LINE)?;
        self.button(MenuItem::Cancel, (40.0, 660.0, 132.0), selected, startup, false)?;
        self.button(MenuItem::Apply, (800.0, 660.0, 200.0), selected, startup, true)?;
        self.text((206.0, 687.0), 15.0, menu::keys(startup), MUTED, false)?;
        if let Some(message) = message {
            self.text((40.0, 627.0), 15.0, message, WARNING, false)?;
        }
        Ok(())
    }

    fn button(
        &mut self,
        item: MenuItem,
        (x, y, width): (f32, f32, f32),
        selected: MenuItem,
        startup: bool,
        primary: bool,
    ) -> Result<()> {
        self.fill(x, y, width, 44.0, if primary { GOLD } else { PANEL })?;
        self.outline(x, y, width, 44.0, if selected == item { INK } else { LINE })?;
        self.text(
            (x + width / 2.0, y + 29.0),
            18.0,
            menu::label(item, startup),
            if primary { ON_GOLD } else { INK },
            true,
        )
    }

    /// The menu item under a window point, and whether it is a row's left arrow.
    pub fn menu_hit(&mut self, at: (i32, i32)) -> Option<(MenuItem, bool)> {
        self.layout();
        let (px, py) = self.physical(at);
        let (scale, left, top) = self.ui;
        let (x, y) = ((px as f32 - left) / scale, (py as f32 - top) / scale);
        let on_left_arrow = (258.0..300.0).contains(&x);
        let rows = MenuItem::ROWS.len() as f32;
        if (40.0..536.0).contains(&x) && (ROW_TOP..ROW_TOP + rows * ROW_PITCH).contains(&y) {
            let row = ((y - ROW_TOP) / ROW_PITCH) as usize;
            if y - ROW_TOP - row as f32 * ROW_PITCH < ROW_HEIGHT {
                return Some((MenuItem::ROWS[row], on_left_arrow));
            }
        }
        let item = match () {
            () if (566.0..610.0).contains(&y) && (40.0..280.0).contains(&x) => MenuItem::Comfort,
            () if (566.0..610.0).contains(&y) && (296.0..536.0).contains(&x) => MenuItem::Original,
            () if (660.0..704.0).contains(&y) && (40.0..172.0).contains(&x) => MenuItem::Cancel,
            () if (660.0..704.0).contains(&y) && (800.0..1000.0).contains(&x) => MenuItem::Apply,
            () => return None,
        };
        Some((item, on_left_arrow))
    }
}
