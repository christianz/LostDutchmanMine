//! The desktop controller: keys, focus and the settings menu in, game input out.

use desktop::controller::Controller;
use desktop::keys::{ALT, Key, default_keycode, key_event, scancode};
use desktop::settings::{self, DisplaySettings, FULLSCREEN};
use engine::InputKind;

fn press(c: &mut Controller, scancode: u16, mods: u16) {
    c.key(Key { scancode, keycode: default_keycode(scancode), mods, repeat: false }, true);
}

fn release(c: &mut Controller, scancode: u16) {
    c.key(Key { scancode, keycode: default_keycode(scancode), mods: 0, repeat: false }, false);
}

fn playing() -> Controller {
    let config = std::env::temp_dir().join(format!("ldm-controller-{}.ini", std::process::id()));
    Controller::new(config, DisplaySettings::default(), false)
}

fn w() -> Key {
    Key { scancode: scancode::W, keycode: default_keycode(scancode::W), mods: 0, repeat: false }
}

#[test]
fn a_held_direction_walks_and_its_release_drops_repeats() {
    let mut c = playing();
    press(&mut c, scancode::W, 0);
    assert_eq!(c.take_inputs(), [InputKind::Directions(1), InputKind::Key(key_event(w()))]);
    release(&mut c, scancode::W);
    assert_eq!(
        c.take_inputs(),
        [InputKind::Directions(0), InputKind::Release(u32::from(scancode::W))]
    );
}

#[test]
fn space_is_a_held_state_and_a_key() {
    let mut c = playing();
    press(&mut c, scancode::SPACE, 0);
    assert_eq!(
        c.take_inputs(),
        [InputKind::Space(true), InputKind::Directions(0), InputKind::Key(0x3920)]
    );
    release(&mut c, scancode::SPACE);
    assert_eq!(
        c.take_inputs(),
        [InputKind::Space(false), InputKind::Directions(0), InputKind::Release(44)]
    );
}

#[test]
fn f11_pauses_into_the_menu_and_escape_resumes() {
    let mut c = playing();
    press(&mut c, scancode::F11, 0);
    assert!(c.paused() && c.menu_open());
    assert_eq!(c.take_inputs(), [InputKind::Clear]);
    press(&mut c, scancode::ESCAPE, 0);
    assert!(!c.paused() && !c.menu_open() && !c.quit());
    assert_eq!(c.take_inputs(), [InputKind::Clear]);
}

#[test]
fn the_startup_menu_pauses_once_the_game_shows_vga() {
    let config = std::env::temp_dir().join(format!("ldm-startup-{}.ini", std::process::id()));
    let mut c = Controller::new(config, DisplaySettings::default(), true);
    c.tick(349, 0x13);
    assert!(!c.paused(), "the game warms up behind the menu");
    c.tick(350, 3);
    assert!(!c.paused());
    c.tick(351, 0x13);
    assert!(c.paused());
    press(&mut c, scancode::ESCAPE, 0);
    assert!(c.quit(), "cancelling the startup menu quits");
}

#[test]
fn apply_saves_the_draft_and_switches_qol() {
    let config = std::env::temp_dir().join(format!("ldm-apply-{}.ini", std::process::id()));
    let mut c = Controller::new(config.clone(), DisplaySettings::default(), true);
    // From the Scaling row, down to the QoL row.
    for _ in 0..5 {
        press(&mut c, scancode::DOWN, 0);
    }
    press(&mut c, scancode::RIGHT, 0);
    c.take_inputs();
    press(&mut c, scancode::RETURN, 0);
    assert_eq!(c.take_inputs(), [InputKind::Qol(false), InputKind::Clear]);
    assert!(!c.menu_open() && !c.settings().qol);
    assert!(!settings::load(&config).qol);
    std::fs::remove_file(config).expect("the saved settings");
}

#[test]
fn alt_enter_toggles_fullscreen_without_typing() {
    let mut c = playing();
    press(&mut c, scancode::RETURN, ALT);
    assert_eq!(c.settings().window, FULLSCREEN);
    assert_eq!(c.take_inputs(), [InputKind::Directions(0), InputKind::Clear]);
    press(&mut c, scancode::RETURN, ALT);
    assert_eq!(c.settings().window, 1, "back to the last window size");
}

#[test]
fn losing_focus_releases_everything() {
    let mut c = playing();
    press(&mut c, scancode::W, 0);
    c.take_inputs();
    c.focus_lost();
    assert_eq!(c.take_inputs(), [InputKind::Clear]);
    press(&mut c, scancode::D, 0);
    assert_eq!(c.take_inputs()[0], InputKind::Directions(8), "W is no longer held");
}
