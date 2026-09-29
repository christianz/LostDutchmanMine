//! Showing the game and the settings menu in the window.
//!
//! The picture itself comes from the `desktop` crate's pure pipeline; this
//! uploads it, stretches it with the renderer, and lays the CRT mask and glow
//! over it. The menu is drawn at the window's physical resolution, with text
//! from the pre-baked `ui-font.bmp`, so it stays sharp on high-DPI screens.
//! The font and the icon are built into the executable.

mod settings_menu;

use anyhow::{Context, Result, anyhow};
use desktop::menu::picture_percent;
use desktop::pixels::{self, Pixels, crt_glow, crt_mask, display_pixels, picture_rect};
use desktop::settings::{Crt, DisplaySettings, FULLSCREEN, Scaling};
use sdl2::VideoSubsystem;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, ScaleMode, Texture, TextureCreator, WindowCanvas};
use sdl2::rwops::RWops;
use sdl2::surface::Surface;
use sdl2::video::{FullscreenType, WindowContext};

const INK: u32 = 0xfff0_e9dd;
const MUTED: u32 = 0xffb9_ad9a;
const GOLD: u32 = 0xffe0_b877;
const PANEL: u32 = 0xff29_2319;
const LINE: u32 = 0xff4b_4030;
const BACKGROUND: u32 = 0xff17_140f;
const ON_GOLD: u32 = 0xff21_1a10;
const WARNING: u32 = 0xffff_ab81;
const BLACK: u32 = 0xff00_0000;

/// The menu is laid out on a 1040x740 canvas, scaled to fit the window.
const LAYOUT: (f32, f32) = (1040.0, 740.0);
/// Settings rows fill the column down to the preset buttons at y=566.
const ROW_TOP: f32 = 154.0;
const ROW_PITCH: f32 = 57.0;
const ROW_HEIGHT: f32 = 46.0;
/// The settings menu's font, baked by `tools/bake_font.py`, and the icon.
const FONT: &[u8] = include_bytes!("../../../../resources/ui-font.bmp");
pub(crate) const ICON: &[u8] = include_bytes!("../../../../resources/ldm-icon.bmp");
/// The three window sizes, smallest first.
const WINDOWS: [(u32, u32); 3] = [(960, 720), (1280, 960), (1600, 1200)];

fn colour(c: u32) -> Color {
    let [alpha, red, green, blue] = c.to_be_bytes();
    Color::RGBA(red, green, blue, alpha)
}

/// The window's renderer and what it draws with.
pub struct Presentation<'a> {
    canvas: WindowCanvas,
    video: VideoSubsystem,
    creator: &'a TextureCreator<WindowContext>,
    font: Texture<'a>,
    icon: Option<Texture<'a>>,
    picture: Option<(Texture<'a>, u32, u32)>,
    glow: Option<Texture<'a>>,
    mask: Option<(Texture<'a>, u32, u32, Crt)>,
    processed: Vec<u32>,
    bytes: Vec<u8>,
    uploaded: Option<(Box<Pixels>, DisplaySettings)>,
    glow_pixels: Box<Pixels>,
    ui: (f32, f32, f32),
}

impl<'a> Presentation<'a> {
    /// Prepares the renderer's textures.
    ///
    /// # Errors
    ///
    /// When the renderer fails.
    pub fn new(
        canvas: WindowCanvas,
        video: VideoSubsystem,
        creator: &'a TextureCreator<WindowContext>,
    ) -> Result<Self> {
        let font = load_texture(creator, FONT).context("the settings font")?;
        let icon = Some(load_texture(creator, ICON).context("the icon")?);
        Ok(Presentation {
            canvas,
            video,
            creator,
            font,
            icon,
            picture: None,
            glow: None,
            mask: None,
            processed: Vec::new(),
            bytes: Vec::new(),
            uploaded: None,
            glow_pixels: pixels::filled(0),
            ui: (1.0, 0.0, 0.0),
        })
    }

    fn display(&self) -> i32 {
        self.canvas.window().display_index().unwrap_or(0).max(0)
    }

    /// The monitor's refresh rate, 60 when unknown.
    pub fn refresh_rate(&self) -> i32 {
        self.video.current_display_mode(self.display()).map_or(60, |mode| mode.refresh_rate).max(1)
    }

    /// Sizes the window for `settings`: desktop fullscreen, or a window scaled
    /// down to fit the usable screen.
    ///
    /// # Errors
    ///
    /// When SDL cannot change the window.
    pub fn resize(&mut self, settings: DisplaySettings) -> Result<()> {
        let window = self.canvas.window_mut();
        if settings.window == FULLSCREEN {
            window.set_fullscreen(FullscreenType::Desktop).map_err(|e| anyhow!(e))?;
        } else {
            window.set_fullscreen(FullscreenType::Off).map_err(|e| anyhow!(e))?;
            let (width, height) = WINDOWS[usize::from(settings.window.min(2))];
            let display = window.display_index().unwrap_or(0).max(0);
            let available =
                self.video.display_usable_bounds(display).unwrap_or(Rect::new(0, 0, 1920, 1080));
            let fit = 1f64
                .min(f64::from(available.width().saturating_sub(32).max(320)) / f64::from(width))
                .min(f64::from(available.height().saturating_sub(64).max(240)) / f64::from(height));
            let window = self.canvas.window_mut();
            window
                .set_size((f64::from(width) * fit) as u32, (f64::from(height) * fit) as u32)
                .map_err(|e| anyhow!(e))?;
        }
        self.canvas.set_viewport(None);
        Ok(())
    }

    fn output_size(&self) -> (i32, i32) {
        let (width, height) = self.canvas.output_size().unwrap_or((1, 1));
        (width as i32, height as i32)
    }

    fn upload(&mut self, frame: &Pixels, settings: DisplaySettings) -> Result<()> {
        let unchanged = self.uploaded.as_ref().is_some_and(|(previous, style)| {
            same_style(*style, settings) && previous.as_slice() == frame.as_slice()
        });
        if unchanged {
            return Ok(());
        }
        let (width, height) = display_pixels(frame, &settings, &mut self.processed);
        if self.picture.as_ref().is_none_or(|&(_, w, h)| (w, h) != (width, height)) {
            let texture = self
                .creator
                .create_texture_streaming(PixelFormatEnum::ARGB8888, width, height)
                .map_err(|e| anyhow!(e))?;
            self.picture = Some((texture, width, height));
        }
        let (texture, ..) = self.picture.as_mut().expect("created above");
        texture.set_scale_mode(if settings.scaling == Scaling::Crisp {
            ScaleMode::Nearest
        } else {
            ScaleMode::Linear
        });
        fill_bytes(&mut self.bytes, &self.processed);
        texture.update(None, &self.bytes, width as usize * 4).map_err(|e| anyhow!(e))?;
        if settings.crt != Crt::Off {
            if self.glow.is_none() {
                let mut glow = self
                    .creator
                    .create_texture_streaming(PixelFormatEnum::ARGB8888, 320, 200)
                    .map_err(|e| anyhow!(e))?;
                glow.set_scale_mode(ScaleMode::Linear);
                glow.set_blend_mode(BlendMode::Add);
                self.glow = Some(glow);
            }
            crt_glow(&self.processed, width, height, &mut self.glow_pixels);
            fill_bytes(&mut self.bytes, self.glow_pixels.as_slice());
            let glow = self.glow.as_mut().expect("created above");
            glow.update(None, &self.bytes, 320 * 4).map_err(|e| anyhow!(e))?;
        }
        let mut copy = pixels::filled(0);
        copy.copy_from_slice(frame);
        self.uploaded = Some((copy, settings));
        Ok(())
    }

    fn picture(&mut self, destination: Rect, settings: DisplaySettings) -> Result<()> {
        let Some((texture, ..)) = &self.picture else { return Ok(()) };
        self.canvas.copy(texture, None, destination).map_err(|e| anyhow!(e))?;
        if settings.crt == Crt::Off {
            return Ok(());
        }
        let (width, height) = (destination.width(), destination.height());
        if self
            .mask
            .as_ref()
            .is_none_or(|&(_, w, h, crt)| (w, h, crt) != (width, height, settings.crt))
        {
            let mut pixels = Vec::new();
            crt_mask(settings.crt, width as i32, height as i32, &mut pixels);
            let mut mask = self
                .creator
                .create_texture_static(PixelFormatEnum::ARGB8888, width, height)
                .map_err(|e| anyhow!(e))?;
            mask.set_blend_mode(BlendMode::Mod);
            mask.set_scale_mode(ScaleMode::Nearest);
            fill_bytes(&mut self.bytes, &pixels);
            mask.update(None, &self.bytes, width as usize * 4).map_err(|e| anyhow!(e))?;
            self.mask = Some((mask, width, height, settings.crt));
        }
        let (mask, ..) = self.mask.as_ref().expect("created above");
        self.canvas.copy(mask, None, destination).map_err(|e| anyhow!(e))?;
        if let Some(glow) = &mut self.glow {
            glow.set_alpha_mod(if settings.crt == Crt::Strong { 30 } else { 18 });
            self.canvas.copy(glow, None, destination).map_err(|e| anyhow!(e))?;
        }
        Ok(())
    }

    /// Draws the game's frame, placed at 4:3.
    ///
    /// # Errors
    ///
    /// When the renderer fails.
    pub fn game(&mut self, frame: &Pixels, settings: DisplaySettings) -> Result<()> {
        self.upload(frame, settings)?;
        let (width, height) = self.output_size();
        let bounds = picture_rect(width, height, picture_percent(&settings));
        self.canvas.set_draw_color(colour(BLACK));
        self.canvas.clear();
        if !bounds.is_empty() {
            let destination = Rect::new(bounds.x, bounds.y, bounds.w as u32, bounds.h as u32);
            self.picture(destination, settings)?;
        }
        Ok(())
    }

    /// Shows what was drawn.
    pub fn present(&mut self) {
        self.canvas.present();
    }

    /// Writes what was drawn, at the renderer's resolution, as a BMP.
    ///
    /// # Errors
    ///
    /// When the pixels cannot be read or the file written.
    pub fn capture(&self, file: &std::path::Path) -> Result<()> {
        let (width, height) = self.canvas.output_size().map_err(|e| anyhow!(e))?;
        let mut pixels =
            self.canvas.read_pixels(None, PixelFormatEnum::ARGB8888).map_err(|e| anyhow!(e))?;
        let surface =
            Surface::from_data(&mut pixels, width, height, width * 4, PixelFormatEnum::ARGB8888)
                .map_err(|e| anyhow!(e))?;
        if let Some(folder) = file.parent().filter(|folder| !folder.as_os_str().is_empty()) {
            std::fs::create_dir_all(folder)?;
        }
        surface.save_bmp(file).map_err(|e| anyhow!("{}: {e}", file.display()))
    }

    /// A window point in the renderer's physical pixels.
    fn physical(&self, (x, y): (i32, i32)) -> (i32, i32) {
        let (window_width, window_height) = self.canvas.window().size();
        let (width, height) = self.output_size();
        let scale = |at: i32, size: i32, window: u32| {
            (i64::from(at) * i64::from(size) / i64::from(window.max(1))) as i32
        };
        (scale(x, width, window_width), scale(y, height, window_height))
    }

    /// The game pixel under a window point: the pixel itself when the point is
    /// on the picture (`true`), otherwise the nearest pixel on its edge.
    pub fn game_point(
        &self,
        at: (i32, i32),
        settings: DisplaySettings,
    ) -> Option<((i32, i32), bool)> {
        let (px, py) = self.physical(at);
        let (width, height) = self.output_size();
        let bounds = picture_rect(width, height, picture_percent(&settings));
        let nearest = pixels::nearest_picture_point(bounds, px, py)?;
        Some((nearest, pixels::picture_point(bounds, px, py).is_some()))
    }
}

/// Two settings draw the same processed picture.
fn same_style(a: DisplaySettings, b: DisplaySettings) -> bool {
    (a.scaling, a.colour, a.brightness, a.crt) == (b.scaling, b.colour, b.brightness, b.crt)
}

/// SDL takes pixel data as bytes, in the machine's own order.
fn fill_bytes(bytes: &mut Vec<u8>, pixels: &[u32]) {
    bytes.clear();
    bytes.extend(pixels.iter().flat_map(|pixel| pixel.to_ne_bytes()));
}

/// A surface from a built-in BMP.
pub(crate) fn bitmap(bytes: &[u8]) -> Result<Surface<'static>> {
    let mut data = RWops::from_bytes(bytes).map_err(|e| anyhow!(e))?;
    Surface::load_bmp_rw(&mut data).map_err(|e| anyhow!(e))
}

fn load_texture<'a>(
    creator: &'a TextureCreator<WindowContext>,
    bytes: &[u8],
) -> Result<Texture<'a>> {
    let surface = bitmap(bytes)?;
    let mut texture = creator.create_texture_from_surface(&surface)?;
    texture.set_blend_mode(BlendMode::Blend);
    texture.set_scale_mode(ScaleMode::Linear);
    Ok(texture)
}
