//! Desktop keys to the game's BIOS keys, as the C++ desktop mapped them.

use desktop::keys::{
    Key, NUM_LOCK, SHIFT, bios, default_keycode, direction, key_event, keycode, menu_key, movement,
    scancode,
};

fn key(scancode: u16, mods: u16) -> Key {
    Key { scancode, keycode: default_keycode(scancode), mods, repeat: false }
}

fn typed(character: char, mods: u16) -> Key {
    Key { scancode: 0, keycode: u32::from(character), mods, repeat: false }
}

#[test]
fn letters_carry_their_scan_and_case() {
    assert_eq!(bios(key(scancode::A, 0)), 0x1e61);
    assert_eq!(bios(key(scancode::A, SHIFT)), 0x1e41);
    assert_eq!(bios(key(scancode::A, desktop::keys::CAPS_LOCK)), 0x1e41);
    assert_eq!(bios(key(scancode::A, SHIFT | desktop::keys::CAPS_LOCK)), 0x1e61);
}

#[test]
fn digits_use_the_top_row_scans_and_shift_symbols() {
    assert_eq!(bios(typed('1', 0)), 0x0231);
    assert_eq!(bios(typed('1', SHIFT)), 0x0221);
    assert_eq!(bios(typed('0', 0)), 0x0b30);
    assert_eq!(bios(typed('0', SHIFT)), 0x0b29);
    assert_eq!(bios(typed('/', 0)), 0x352f);
}

#[test]
fn the_keypad_types_digits_only_with_num_lock() {
    assert_eq!(bios(key(scancode::KP_1, NUM_LOCK)), 0x0231);
    assert_eq!(bios(key(scancode::KP_0, NUM_LOCK)), 0x0b30);
    assert_eq!(bios(key(scancode::KP_1, NUM_LOCK | SHIFT)), 0x4f00, "shift inverts Num Lock");
    assert_eq!(bios(key(scancode::KP_1, 0)), 0x4f00);
    assert_eq!(bios(key(scancode::KP_5, 0)), 0x4c00);
    assert_eq!(bios(key(scancode::KP_ENTER, 0)), 0x1c0d);
}

#[test]
fn named_keys_have_their_bios_pairs() {
    assert_eq!(bios(key(scancode::SPACE, 0)), 0x3920);
    assert_eq!(bios(key(scancode::ESCAPE, 0)), 0x011b);
    assert_eq!(bios(key(scancode::RETURN, 0)), 0x1c0d);
    assert_eq!(bios(key(scancode::RIGHT, 0)), 0x4d00);
    assert_eq!(bios(key(scancode::F6, 0)), 0x4000);
    assert_eq!(bios(key(scancode::F11, 0)), 0, "F11 is the desktop's own");
}

#[test]
fn movement_keys_are_tagged_with_their_scan_and_physical_key() {
    let w = key(scancode::W, 0);
    assert_eq!(key_event(w), 0x1177 | 0x48 << 17 | u32::from(scancode::W) << 24);
    let repeat = Key { repeat: true, ..w };
    assert_eq!(key_event(repeat), key_event(w) | 1 << 16);
    assert_eq!(
        key_event(key(scancode::KP_0, 0)),
        0x5200 | 0x52 << 17 | u32::from(scancode::KP_0) << 24
    );
    assert_eq!(key_event(key(scancode::SPACE, 0)), 0x3920, "only directions carry tags");
}

#[test]
fn held_directions_combine_and_opposites_cancel() {
    let mut held = [false; 512];
    held[usize::from(scancode::W)] = true;
    held[usize::from(scancode::D)] = true;
    assert_eq!(movement(&held), 9);
    held[usize::from(scancode::S)] = true;
    assert_eq!(movement(&held), 8, "up and down cancel on their own axis");
    assert_eq!(direction(scancode::HOME), 5);
    assert_eq!(direction(scancode::KP_3), 10);
}

#[test]
fn directions_navigate_the_menu() {
    assert_eq!(menu_key(key(scancode::W, 0)), keycode::UP);
    assert_eq!(menu_key(key(scancode::KP_6, 0)), keycode::RIGHT);
    assert_eq!(menu_key(key(scancode::ESCAPE, 0)), keycode::ESCAPE);
}
