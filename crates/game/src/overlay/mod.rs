//! The QoL overlay: a hover toolbar over the original panel, hover outlines on
//! a scene's action buttons, and "sold out" on mules already bought.
//!
//! It reuses the player's own panel artwork and eight-pixel lettering, and it
//! never writes into the original framebuffer or save data. Hooks tell it which
//! buttons the current scene offers and when a dialog covers them.

mod draw;

use engine::Stop;
use machine::Machine;
use patches::After;

use crate::Game;
use crate::assets::decode_asset;
use crate::symbols::{FONT, PANEL_ARTWORK, VIDEO_MODE_VGA};

pub(crate) use draw::{draw, pointer_visible};

/// One page of the panel atlas: 320x200 pixels.
const ATLAS_SIZE: usize = 64_000;
/// The font: 256 glyphs of eight rows.
pub(crate) const FONT_SIZE: usize = 2048;
/// A scene offers up to four action buttons.
const ACTIONS: u16 = 4;
/// The six toolbar buttons.
const TOOLS: i32 = 6;

/// A button's rectangle, in game pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Button {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl Button {
    fn contains(self, x: i32, y: i32) -> bool {
        (self.x..self.x + self.width).contains(&x) && (self.y..self.y + self.height).contains(&y)
    }

    /// Toolbar button `i`: Cash, Life, Food, Tools, Ammo, Game.
    const fn tool(i: i32) -> Self {
        Button { x: 52 + i * 42, y: 167, width: 40, height: 32 }
    }

    /// Action button `i` of a scene, in a two-by-two grid.
    const fn action(i: i32) -> Self {
        Button {
            x: if i < 2 { 72 } else { 152 },
            y: if i % 2 == 1 { 138 } else { 118 },
            width: 75,
            height: 19,
        }
    }
}

/// What the overlay knows about the screen.
#[derive(Clone, Debug, Default)]
pub(crate) struct Overlay {
    /// The game's 8x8 font, copied from the image at boot.
    pub(crate) font: Vec<u8>,
    /// The panel artwork's colour indices, once loaded.
    atlas: Option<Box<[u8]>>,
    /// Bit `i`: the scene offers action button `i`.
    pub(crate) context_buttons: u8,
    /// Scene buttons covered by open menus, innermost last.
    covered: Vec<u8>,
    /// Inside the mule shop, and whether its offers are on screen.
    pub(crate) mule_shop: bool,
    pub(crate) mule_shop_visible: bool,
}

impl Overlay {
    /// Whether a dialog or menu covers the scene.
    pub(crate) fn menu_open(&self) -> bool {
        !self.covered.is_empty()
    }

    /// Copies the game's font from the unrelocated image.
    pub(crate) fn load_font(&mut self, image: &[u8]) {
        let start = usize::from(FONT.segment) * 16 + usize::from(FONT.offset);
        self.font = image.get(start..start + FONT_SIZE).map(<[u8]>::to_vec).unwrap_or_default();
    }

    /// Dialogs save and replace the original pixels; hover targets follow the
    /// same lifetime, including dialogs that draw their buttons directly.
    fn open_menu(&mut self) {
        self.covered.push(self.context_buttons);
        self.context_buttons = 0;
    }

    fn close_menu(&mut self) {
        if let Some(buttons) = self.covered.pop() {
            self.context_buttons = buttons;
        }
    }
}

/// The original panel is on screen: load its artwork for the toolbar.
pub(crate) fn prepare_panel(g: &mut Game) -> Result<After, Stop> {
    if g.overlay.atlas.is_some() || g.dos.video.mode != VIDEO_MODE_VGA {
        return Ok(After::Continue);
    }
    let failed = |why: String| Stop::Program(format!("the original VGA panel artwork: {why}"));
    let path =
        g.dos.files.resolve(PANEL_ARTWORK, false).map_err(|error| failed(error.to_string()))?;
    let packed =
        std::fs::read(&path).map_err(|error| failed(format!("{}: {error}", path.display())))?;
    let decoded = decode_asset(&packed).map_err(|error| failed(error.to_string()))?;
    if decoded.len() < ATLAS_SIZE {
        return Err(failed("incomplete".to_owned()));
    }
    // The atlas still uses the 16-colour drawing path: the original blitter masks
    // each byte to its low nibble, and the upper bits are not palette indices.
    g.overlay.atlas = Some(decoded[..ATLAS_SIZE].iter().map(|&colour| colour & 15).collect());
    Ok(After::Continue)
}

/// The scene's drawing routine takes one flag pointer per action button.
pub(crate) fn read_context_buttons(m: &Machine, g: &mut Game) -> After {
    let (ss, sp, ds) = (m.regs.ss, m.regs.sp, m.regs.ds);
    g.overlay.context_buttons = (0..ACTIONS)
        .filter(|&i| {
            let flag = m.memory.read16(ss, sp.wrapping_add(4 + i * 2));
            m.memory.read8(ds, flag) != 0
        })
        .fold(0, |buttons, i| buttons | 1 << i);
    After::Continue
}

/// The scene's buttons are gone.
pub(crate) fn clear_context_buttons(g: &mut Game) -> After {
    g.overlay.context_buttons = 0;
    After::Continue
}

/// The shared selector opens: toolbar and hover clicks become clicks on the
/// original panel buttons, then its menu covers the scene.
pub(crate) fn open_selector_menu(m: &mut Machine, g: &mut Game) -> After {
    remap_click(m, g);
    g.overlay.open_menu();
    g.overlay.mule_shop_visible = false;
    After::Continue
}

/// The selector's click is its first two arguments. Only panel targets are
/// remapped; building item coordinates pass through.
fn remap_click(m: &mut Machine, g: &Game) {
    if !g.qol || g.dos.video.mode != VIDEO_MODE_VGA {
        return;
    }
    let (ss, sp) = (m.regs.ss, m.regs.sp);
    let (x, y) = (m.memory.read16(ss, sp.wrapping_add(4)), m.memory.read16(ss, sp.wrapping_add(6)));
    let (x, y) = (i32::from(x), i32::from(y));
    let original = (0..TOOLS)
        .find(|&i| Button::tool(i).contains(x, y))
        .map(|i| (69 + i * 42, 181))
        .or_else(|| {
            (0..4)
                .find(|&i| {
                    g.overlay.context_buttons & 1 << i != 0 && Button::action(i).contains(x, y)
                })
                .map(|i| (if i < 2 { 108 } else { 188 }, if i % 2 == 1 { 147 } else { 127 }))
        });
    if let Some((x, y)) = original {
        m.memory.write16(ss, sp.wrapping_add(4), x as u16);
        m.memory.write16(ss, sp.wrapping_add(6), y as u16);
    }
}

/// A building's menu opens.
pub(crate) fn open_building_menu(g: &mut Game) -> After {
    g.overlay.open_menu();
    g.overlay.mule_shop_visible = false;
    After::Continue
}

/// A menu opens without hover targets underneath, such as the saloon sleep.
pub(crate) fn open_menu(g: &mut Game) -> After {
    g.overlay.open_menu();
    After::Continue
}

/// The innermost menu closes.
pub(crate) fn close_menu(g: &mut Game) -> After {
    g.overlay.close_menu();
    After::Continue
}

/// The mule shop opens.
pub(crate) fn enter_mule_shop(g: &mut Game) -> After {
    g.overlay.mule_shop = true;
    After::Continue
}

/// The mule shop closes.
pub(crate) fn leave_mule_shop(g: &mut Game) -> After {
    g.overlay.mule_shop = false;
    g.overlay.mule_shop_visible = false;
    After::Continue
}

/// The mule shop draws its offers.
pub(crate) fn show_mule_shop(g: &mut Game) -> After {
    g.overlay.mule_shop_visible = g.overlay.mule_shop;
    After::Continue
}

/// A dialog covers the mule shop's offers.
pub(crate) fn hide_mule_shop(g: &mut Game) -> After {
    g.overlay.mule_shop_visible = false;
    After::Continue
}
