//! Desktop keys, as SDL names them, to the keys the game reads.
//!
//! Scancodes are physical keys and keycodes what the layout makes of them; both
//! are SDL's own numbers, so a window passes them straight through and scripts
//! name scancodes alone. A key the game reads is its BIOS character and scan,
//! tagged as the `dos` keyboard documents.

use dos::{KEY_MOVEMENT_SHIFT, KEY_REPEAT, KEY_SOURCE_SHIFT};

/// A key as SDL reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    /// The physical key.
    pub scancode: u16,
    /// What the keyboard layout makes of it.
    pub keycode: u32,
    /// The modifier keys and locks held.
    pub mods: u16,
    /// Whether this is an auto-repeat.
    pub repeat: bool,
}

/// Either Shift.
pub const SHIFT: u16 = 0x0003;
/// Either Alt.
pub const ALT: u16 = 0x0300;
/// Num Lock is on.
pub const NUM_LOCK: u16 = 0x1000;
/// Caps Lock is on.
pub const CAPS_LOCK: u16 = 0x2000;
/// One more than the highest scancode.
pub const SCANCODES: usize = 512;

/// SDL's scancodes for the keys the game uses.
#[allow(missing_docs, reason = "each is the key it names")]
pub mod scancode {
    pub const A: u16 = 4;
    pub const D: u16 = 7;
    pub const S: u16 = 22;
    pub const W: u16 = 26;
    pub const Z: u16 = 29;
    pub const ONE: u16 = 30;
    pub const ZERO: u16 = 39;
    pub const RETURN: u16 = 40;
    pub const ESCAPE: u16 = 41;
    pub const BACKSPACE: u16 = 42;
    pub const TAB: u16 = 43;
    pub const SPACE: u16 = 44;
    pub const F1: u16 = 58;
    pub const F6: u16 = 63;
    pub const F11: u16 = 68;
    pub const F12: u16 = 69;
    pub const INSERT: u16 = 73;
    pub const HOME: u16 = 74;
    pub const PAGE_UP: u16 = 75;
    pub const END: u16 = 77;
    pub const PAGE_DOWN: u16 = 78;
    pub const RIGHT: u16 = 79;
    pub const LEFT: u16 = 80;
    pub const DOWN: u16 = 81;
    pub const UP: u16 = 82;
    pub const KP_ENTER: u16 = 88;
    pub const KP_1: u16 = 89;
    pub const KP_2: u16 = 90;
    pub const KP_3: u16 = 91;
    pub const KP_4: u16 = 92;
    pub const KP_5: u16 = 93;
    pub const KP_6: u16 = 94;
    pub const KP_7: u16 = 95;
    pub const KP_8: u16 = 96;
    pub const KP_9: u16 = 97;
    pub const KP_0: u16 = 98;
}

/// SDL's keycodes for the keys the controller distinguishes. Keys without a
/// character are their scancode with bit 30 set.
#[allow(missing_docs, reason = "each is the key it names")]
pub mod keycode {
    use super::scancode;

    const fn named(scancode: u16) -> u32 {
        scancode as u32 | 1 << 30
    }

    pub const RETURN: u32 = 13;
    pub const ESCAPE: u32 = 27;
    pub const BACKSPACE: u32 = 8;
    pub const TAB: u32 = 9;
    pub const SPACE: u32 = 32;
    pub const DELETE: u32 = 127;
    pub const F1: u32 = named(scancode::F1);
    pub const F11: u32 = named(scancode::F11);
    pub const INSERT: u32 = named(scancode::INSERT);
    pub const HOME: u32 = named(scancode::HOME);
    pub const PAGE_UP: u32 = named(scancode::PAGE_UP);
    pub const END: u32 = named(scancode::END);
    pub const PAGE_DOWN: u32 = named(scancode::PAGE_DOWN);
    pub const RIGHT: u32 = named(scancode::RIGHT);
    pub const LEFT: u32 = named(scancode::LEFT);
    pub const DOWN: u32 = named(scancode::DOWN);
    pub const UP: u32 = named(scancode::UP);
    pub const KP_ENTER: u32 = named(scancode::KP_ENTER);
    pub const KP_1: u32 = named(scancode::KP_1);
    pub const KP_2: u32 = named(scancode::KP_2);
    pub const KP_3: u32 = named(scancode::KP_3);
    pub const KP_4: u32 = named(scancode::KP_4);
    pub const KP_6: u32 = named(scancode::KP_6);
    pub const KP_7: u32 = named(scancode::KP_7);
    pub const KP_8: u32 = named(scancode::KP_8);
    pub const KP_9: u32 = named(scancode::KP_9);
    pub const KP_0: u32 = named(scancode::KP_0);

    /// Any key without a character.
    pub const fn of(scancode: u16) -> u32 {
        named(scancode)
    }
}

/// The punctuation row after the digits, in scancode order from 45.
const PUNCTUATION: &[u8; 12] = b"-=[]\\#;'`,./";

/// The keycode SDL's default US layout gives a scancode, as a window without a
/// keyboard, or a script, sees it.
pub fn default_keycode(scancode: u16) -> u32 {
    match scancode {
        scancode::A..=scancode::Z => u32::from(b'a') + u32::from(scancode - scancode::A),
        scancode::ONE..=38 => u32::from(b'1') + u32::from(scancode - scancode::ONE),
        scancode::ZERO => u32::from(b'0'),
        scancode::RETURN => keycode::RETURN,
        scancode::ESCAPE => keycode::ESCAPE,
        scancode::BACKSPACE => keycode::BACKSPACE,
        scancode::TAB => keycode::TAB,
        scancode::SPACE => keycode::SPACE,
        45..=56 => u32::from(PUNCTUATION[usize::from(scancode - 45)]),
        76 => keycode::DELETE,
        0..=3 => 0,
        _ => keycode::of(scancode),
    }
}

/// The movement a physical key means: 1 up, 2 down, 4 left, 8 right, or a
/// diagonal. Arrows, WASD and the keypad all walk.
pub const fn direction(scancode: u16) -> u8 {
    match scancode {
        scancode::UP | scancode::KP_8 | scancode::W => 1,
        scancode::DOWN | scancode::KP_2 | scancode::S => 2,
        scancode::LEFT | scancode::KP_4 | scancode::A => 4,
        scancode::RIGHT | scancode::KP_6 | scancode::D => 8,
        scancode::HOME | scancode::KP_7 => 5,
        scancode::PAGE_UP | scancode::KP_9 => 9,
        scancode::END | scancode::KP_1 => 6,
        scancode::PAGE_DOWN | scancode::KP_3 => 10,
        _ => 0,
    }
}

/// Up and down.
const VERTICAL: u8 = 0b0011;
/// Left and right.
const HORIZONTAL: u8 = 0b1100;

/// The directions all held keys make; opposites cancel, per axis.
pub fn movement(held: &[bool; SCANCODES]) -> u8 {
    let mut mask = (0..)
        .zip(held)
        .filter(|&(_, &down)| down)
        .fold(0, |mask, (scancode, _)| mask | direction(scancode));
    for axis in [VERTICAL, HORIZONTAL] {
        if mask & axis == axis {
            mask &= !axis;
        }
    }
    mask
}

/// The BIOS scans of the keypad's digits, 0 to 9, without Num Lock.
const KEYPAD_SCANS: [u16; 10] = [0x52, 0x4f, 0x50, 0x51, 0x4b, 0x4c, 0x4d, 0x47, 0x48, 0x49];
/// The BIOS scans of the letters, a to z.
const LETTER_SCANS: [u16; 26] = [
    0x1e, 0x30, 0x2e, 0x20, 0x12, 0x21, 0x22, 0x23, 0x17, 0x24, 0x25, 0x26, 0x32, 0x31, 0x18, 0x19,
    0x10, 0x13, 0x1f, 0x14, 0x16, 0x2f, 0x11, 0x2d, 0x15, 0x2c,
];
/// The digits' shifted symbols, 1 to 9.
const SHIFTED_DIGITS: &[u8; 9] = b"!@#$%^&*(";

/// The BIOS character and scan pair for a key. Keypad identities are physical,
/// so they hold across Num Lock and layouts; digits use the top-row scans,
/// since the save selector examines scans rather than characters.
pub fn bios(key: Key) -> u16 {
    let held = |mask: u16| key.mods & mask != 0;
    if (scancode::KP_1..=scancode::KP_0).contains(&key.scancode) {
        let digit =
            if key.scancode == scancode::KP_0 { 0 } else { key.scancode - scancode::KP_1 + 1 };
        if held(NUM_LOCK) != held(SHIFT) {
            let scan = if digit == 0 { 0x0b } else { digit + 1 };
            return scan << 8 | (u16::from(b'0') + digit);
        }
        return KEYPAD_SCANS[usize::from(digit)] << 8;
    }
    if key.scancode == scancode::KP_ENTER {
        return 0x1c0d;
    }
    let scan = match key.keycode {
        keycode::ESCAPE => return 0x011b,
        keycode::RETURN | keycode::KP_ENTER => return 0x1c0d,
        keycode::BACKSPACE => return 0x0e08,
        keycode::TAB => return 0x0f09,
        keycode::SPACE => return 0x3920,
        keycode::UP | keycode::KP_8 => 0x48,
        keycode::DOWN | keycode::KP_2 => 0x50,
        keycode::LEFT | keycode::KP_4 => 0x4b,
        keycode::RIGHT | keycode::KP_6 => 0x4d,
        keycode::HOME | keycode::KP_7 => 0x47,
        keycode::PAGE_UP | keycode::KP_9 => 0x49,
        keycode::END | keycode::KP_1 => 0x4f,
        keycode::PAGE_DOWN | keycode::KP_3 => 0x51,
        keycode::INSERT | keycode::KP_0 => 0x52,
        // F1 to F10.
        code if (keycode::F1..keycode::F1 + 10).contains(&code) => {
            0x3b + (code - keycode::F1) as u16
        }
        code @ 32..=126 => return printable(code as u8, held(SHIFT), held(CAPS_LOCK)),
        _ => 0,
    };
    scan << 8
}

fn printable(character: u8, shift: bool, caps: bool) -> u16 {
    let (scan, character) = match character {
        b'a'..=b'z' => {
            let scan = LETTER_SCANS[usize::from(character - b'a')];
            (scan, if shift == caps { character } else { character.to_ascii_uppercase() })
        }
        b'1'..=b'9' => {
            let index = character - b'1';
            (
                u16::from(index) + 2,
                if shift { SHIFTED_DIGITS[usize::from(index)] } else { character },
            )
        }
        b'0' => (0x0b, if shift { b')' } else { character }),
        b'-' => (0x0c, character),
        b'=' => (0x0d, character),
        b'[' => (0x1a, character),
        b']' => (0x1b, character),
        b';' => (0x27, character),
        b'\'' => (0x28, character),
        b'`' => (0x29, character),
        b'\\' => (0x2b, character),
        b',' => (0x33, character),
        b'.' => (0x34, character),
        b'/' => (0x35, character),
        _ => (0, character),
    };
    scan << 8 | u16::from(character)
}

/// The key the game receives: its BIOS pair, plus, for movement keys, the
/// movement scan movement readers see and the physical key, so its release
/// can drop queued repeats.
pub fn key_event(key: Key) -> u32 {
    let movement: u32 = match direction(key.scancode) {
        1 => 0x48,
        2 => 0x50,
        4 => 0x4b,
        8 => 0x4d,
        5 => 0x47,
        9 => 0x49,
        6 => 0x4f,
        10 => 0x51,
        // The keypad's 0 is the original Insert, the action key.
        _ if key.scancode == scancode::KP_0 => 0x52,
        _ => 0,
    };
    let source = if movement == 0 { 0 } else { u32::from(key.scancode) << KEY_SOURCE_SHIFT };
    let repeat = if key.repeat { KEY_REPEAT } else { 0 };
    u32::from(bios(key)) | movement << KEY_MOVEMENT_SHIFT | source | repeat
}

/// The key the settings menu sees: every direction key navigates.
pub fn menu_key(key: Key) -> u32 {
    match direction(key.scancode) {
        1 => keycode::UP,
        2 => keycode::DOWN,
        4 => keycode::LEFT,
        8 => keycode::RIGHT,
        _ => key.keycode,
    }
}
