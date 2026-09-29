//! The C++ `keyboard` test: desktop keys through the original movement and text
//! readers. WASD walks and still types with its case, the keypad walks with Num
//! Lock on or off and types digits with it, and a single key event never
//! replaces a held diagonal. How desktop keys become BIOS keys and held
//! directions is `crates/desktop/tests/keys.rs`; the one combination it lacks
//! is here.

use desktop::keys::{
    CAPS_LOCK, Key, NUM_LOCK, SCANCODES, SHIFT, default_keycode, key_event, movement, scancode,
};
use game::symbols::{Global, PENDING_DIRECTION};
use machine::Address;
use testkit::harness::{Harness, STACK_TOP};

/// The original movement reader, taking one argument.
const READ_MOVEMENT: Address = Address::new(0x0fa7, 0x0066);
/// The original text reader.
const READ_TEXT: Address = Address::new(0x0fa7, 0x003a);
/// The scan of the last key the movement reader passed on as a command.
const COMMAND_SCAN: Global = Global(0x0a6a);
/// A reader returns well within this many steps.
const READ_LIMIT: u64 = 2000;
/// What the movement reader returns for the action key.
const ACTION: u16 = 0x80;

/// The WASD keys.
const WASD: [u16; 4] = [scancode::W, scancode::A, scancode::S, scancode::D];
/// Their letters.
const LETTERS: &[u8; 4] = b"wasd";
/// Their directions: up, left, down, right.
const WASD_DIRECTIONS: [u16; 4] = [1, 4, 2, 8];
/// The keypad from 1 to 9, then 0: diagonals, the idle 5 and the action key.
const KEYPAD_DIRECTIONS: [u16; 10] = [6, 2, 10, 4, 0, 8, 5, 1, 9, ACTION];

/// The key the game receives for a desktop key.
fn key(scancode: u16, keycode: u32, mods: u16, repeat: bool) -> u32 {
    key_event(Key { scancode, keycode, mods, repeat })
}

/// Reads `event` through the original movement or text reader while the
/// desktop holds `held`, and returns what the reader returns.
fn read(h: &mut Harness, event: u32, movement: bool, held: u8) -> u16 {
    h.keyboard().clear();
    h.keyboard().push(event);
    h.set_movement(held);
    h.set(PENDING_DIRECTION, 0);
    if movement {
        h.call(READ_MOVEMENT, &[1]);
    } else {
        h.call(READ_TEXT, &[]);
    }
    h.until(READ_LIMIT, "Original input helper failed to return", Harness::returned);
    if movement {
        h.m.regs.sp += 2;
    }
    assert_eq!(h.m.regs.sp, STACK_TOP, "Keyboard read corrupted stack");
    assert!(!h.game.dos.keyboard.movement_aliases, "Movement aliases leaked into text input");
    h.m.regs.ax
}

/// The loaded game, its movement reader attached to the joystick port.
fn readers() -> Harness {
    let mut h = Harness::loaded();
    h.attach_joystick();
    h
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn wasd_walks_and_types_with_its_case() {
    let mut h = readers();
    for ((&scancode, &letter), direction) in WASD.iter().zip(LETTERS).zip(WASD_DIRECTIONS) {
        for mods in [0, SHIFT, CAPS_LOCK, SHIFT | CAPS_LOCK] {
            let event = key(scancode, u32::from(letter), mods, false);
            assert_eq!(
                read(&mut h, event, true, 0),
                direction,
                "WASD tap did not reach the original movement handler"
            );
            let upper = (mods & SHIFT != 0) != (mods & CAPS_LOCK != 0);
            let expected = if upper { letter.to_ascii_uppercase() } else { letter };
            assert_eq!(
                read(&mut h, event, false, 0) as u8,
                expected,
                "WASD letters or capitalization lost in text input"
            );
        }
    }
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn the_keypad_walks_with_num_lock_either_way_and_types_digits_with_it() {
    let mut h = readers();
    for (scancode, direction) in (scancode::KP_1..=scancode::KP_0).zip(KEYPAD_DIRECTIONS) {
        for mods in [0, NUM_LOCK] {
            let event = key(scancode, 0, mods, false);
            assert_eq!(
                read(&mut h, event, true, 0),
                direction,
                "Numpad movement changed with Num Lock"
            );
            if mods == NUM_LOCK {
                let digit = (scancode - scancode::KP_1 + 1) % 10;
                let top_row_scan = if digit == 0 { 0x0b } else { digit + 1 };
                assert_eq!(
                    read(&mut h, event, false, 0),
                    top_row_scan << 8 | (u16::from(b'0') + digit),
                    "Numeric keypad must select save slots and enter digits"
                );
            }
        }
    }
    let enter = key(scancode::KP_ENTER, 0, NUM_LOCK, false);
    assert_eq!(read(&mut h, enter, false, 0), 0x1c0d, "Numpad Enter did not confirm text input");
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn a_single_key_event_never_replaces_a_held_diagonal() {
    let mut h = readers();
    // The original reader starts with the held mask, then used to replace it
    // with the last single key's scan, including an OS auto-repeat.
    for vertical in [0, 2] {
        for horizontal in [1, 3] {
            for last in [vertical, horizontal] {
                for repeat in [false, true] {
                    let mut held = [false; SCANCODES];
                    held[usize::from(WASD[vertical])] = true;
                    held[usize::from(WASD[horizontal])] = true;
                    let letter = LETTERS[last];
                    let event = key(WASD[last], u32::from(letter), 0, repeat);
                    let mask = movement(&held);
                    assert_eq!(
                        read(&mut h, event, true, mask),
                        u16::from(mask),
                        "An individual key event overrides a held diagonal"
                    );
                    assert_eq!(
                        read(&mut h, event, false, mask) as u8,
                        letter,
                        "Held combinations leaked into text entry"
                    );
                }
            }
        }
    }
    let space = key(scancode::SPACE, default_keycode(scancode::SPACE), 0, false);
    assert_eq!(read(&mut h, space, true, 5), ACTION, "Held movement swallowed Space");
    let f2 = scancode::F1 + 1;
    let f2 = key(f2, default_keycode(f2), 0, false);
    assert!(
        read(&mut h, f2, true, 5) == 0 && h.get(COMMAND_SCAN) == 0x3c,
        "Held movement swallowed a status-menu key"
    );
}

#[test]
fn releasing_one_alias_keeps_another_held_key() {
    let mut held = [false; SCANCODES];
    for scancode in [scancode::W, scancode::S, scancode::D, scancode::KP_6] {
        held[usize::from(scancode)] = true;
    }
    held[usize::from(scancode::D)] = false;
    assert_eq!(movement(&held), 8, "Releasing one alias must preserve another held key");
}
