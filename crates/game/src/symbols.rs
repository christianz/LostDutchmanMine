//! Every address in the original game that the port reads or writes, named.
//!
//! Globals live in the game's data segment. DS points there whenever the
//! game's own code runs, so hooks, which run inside that code, reach globals
//! through DS, exactly as the original instructions do. Frames and reports are
//! built between runs and read the data segment directly.

use machine::{Address, LOAD_SEGMENT, Machine};

/// The game's data segment, image-relative.
pub const DATA_SEGMENT: u16 = 0x72bd;

/// A word in the data segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Global(pub u16);

impl Global {
    /// Reads it through DS, as the game's code does.
    pub fn get(self, m: &Machine) -> u16 {
        m.memory.read16(m.regs.ds, self.0)
    }

    /// Writes it through DS.
    pub fn set(self, m: &mut Machine, value: u16) {
        m.memory.write16(m.regs.ds, self.0, value);
    }

    /// Reads it from the data segment itself, between runs.
    pub fn at_rest(self, m: &Machine) -> u16 {
        m.memory.read16(DATA_SEGMENT + LOAD_SEGMENT, self.0)
    }

    /// The word `index` words further on.
    #[must_use]
    pub const fn nth(self, index: u16) -> Global {
        Global(self.0 + index * 2)
    }
}

// Where the player is.
/// The player's position, and during an encounter the sight's top-left corner.
pub const POSITION_X: Global = Global(0x5b4a);
/// See [`POSITION_X`].
pub const POSITION_Y: Global = Global(0x5b4c);
/// The world map's scroll.
pub const MAP_SCROLL_X: Global = Global(0x5b56);
/// See [`MAP_SCROLL_X`].
pub const MAP_SCROLL_Y: Global = Global(0x5b58);
/// The town page.
pub const TOWN_PAGE: Global = Global(0x5b5a);
/// The building the player is in, or zero.
pub const BUILDING: Global = Global(0x5b5e);
/// Where the player returns to after a building, cave or river.
pub const RETURN_X: Global = Global(0x5b60);
/// See [`RETURN_X`].
pub const RETURN_Y: Global = Global(0x5b62);
/// Set while the scene came from a loaded game rather than a walk.
pub const LOADED_SCENE: Global = Global(0x5b86);

// Which scene is showing: one flag each.
/// The map.
pub const SCENE_MAP: Global = Global(0x5e06);
/// A cave.
pub const SCENE_CAVE: Global = Global(0x5e0a);
/// An encounter.
pub const SCENE_ENCOUNTER: Global = Global(0x5e0c);

// Supplies and survival.
/// Pans carried, a counter that can go stale after a rejected purchase.
pub const PANS: Global = Global(0x53dc);
/// Guns carried.
pub const GUNS: Global = Global(0x53e0);
/// Bullets carried.
pub const BULLETS: Global = Global(0x53e2);
/// Bags of gold carried.
pub const GOLD_BAGS: Global = Global(0x53ea);
/// Cash, low and high words.
pub const CASH: Global = Global(0x53f0);
/// The survival clock's step counter.
pub const SURVIVAL_TICKS: Global = Global(0x5406);
/// Whether each of the three mules is owned.
pub const MULES_OWNED: Global = Global(0x5d5a);
/// The assay office's weight and grade of the bag being sold.
pub const ASSAY_POUNDS: Global = Global(0x5b82);
/// See [`ASSAY_POUNDS`].
pub const ASSAY_GRADE: Global = Global(0x59d0);
/// Pick strokes made in the current mining action.
pub const MINING_STROKES: Global = Global(0x093a);

/// The inventory: 11 slots of four interleaved rows (the player, then each
/// mule); slot 0 holds the carrier's icon.
pub const INVENTORY: Global = Global(0x500e);
/// Inventory slots, including the carrier's icon.
pub const INVENTORY_SLOTS: u16 = 11;
/// An empty inventory slot.
pub const ITEM_NONE: u16 = 0x2b;
/// A pan.
pub const ITEM_PAN: u16 = 0x0f;

/// The inventory cell for `slot` of `row`.
pub const fn inventory(slot: u16, row: u16) -> Global {
    Global(INVENTORY.0 + slot * 8 + row * 2)
}

// Input.
/// 1 while walking or aiming with the keyboard, 0 in hand-cursor mode.
pub const MOUSE_MODE: Global = Global(0x5d62);
/// A direction or command forwarded from the hand-cursor loop.
pub const PENDING_DIRECTION: Global = Global(0x5a1a);
/// The joystick port's direction bits, which the original movement code reads.
pub const JOYSTICK: Address = Address::new(0x72ba, 0x0001);

// Encounters.
/// Set once an encounter is won; its victory choices follow.
pub const ENCOUNTER_WON: Global = Global(0x5302);
/// How far above row 94 the encounter's aiming area ends.
pub const AIM_FLOOR: Global = Global(0x0112);
/// The segment of the original sprite page holding the sight.
pub const SPRITE_PAGE: Address = Address::new(0x1b14, 0xfb37);

/// The original 8x8 font: 256 glyphs of eight rows.
pub const FONT: Address = Address::new(0x1a94, 0x0000);

// Display.
/// The BIOS mode the game plays in: 320x200 in 256 colours.
pub const VIDEO_MODE_VGA: u8 = 0x13;
/// The BIOS modes of the CGA version: 320x200 in four colours, and its grey twin.
pub const VIDEO_MODE_CGA: u8 = 4;
/// See [`VIDEO_MODE_CGA`].
pub const VIDEO_MODE_CGA_GREY: u8 = 5;
/// The VGA framebuffer (mode 13h), linear.
pub const VGA_FRAMEBUFFER: usize = 0xa_0000;
/// The CGA framebuffer (modes 4 and 5), linear.
pub const CGA_FRAMEBUFFER: usize = 0xb_8000;
/// The original panel artwork the QoL toolbar is drawn from.
pub const PANEL_ARTWORK: &[u8] = b"LDMG/PANL_VGA.ZZZ";

/// AX for the graphics selector's accepted VGA choice.
pub const VGA_CHOICE: u16 = 0x1333;

/// Writes the joystick's direction bits.
pub fn set_joystick(m: &mut Machine, directions: u8) {
    m.memory.write8(JOYSTICK.runtime_segment(), JOYSTICK.offset, directions);
}

/// Reads the joystick's direction bits.
pub fn joystick(m: &Machine) -> u8 {
    m.memory.read8(JOYSTICK.runtime_segment(), JOYSTICK.offset)
}
