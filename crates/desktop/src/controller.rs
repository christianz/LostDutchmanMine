//! What desktop input means: keys, focus and the settings menu in, game input
//! and the settings the picture follows out.
//!
//! The controller holds the keys down, so held directions combine and opposites
//! cancel, and the settings menu, which pauses the game while it is open. At
//! startup the menu opens over a game that keeps running until it first shows
//! its VGA screen, so the menu has the title behind it.

use std::path::PathBuf;

use engine::InputKind;

use crate::keys::{self, ALT, Key, SCANCODES, SHIFT, keycode, scancode};
use crate::menu::{self, MenuItem};
use crate::settings::{self, DisplaySettings, FULLSCREEN};

/// The BIOS mode of the game's own screens.
const VGA_MODE: u8 = 0x13;
/// The startup menu lets the game run this long, and until it shows VGA.
const WARMUP_MS: u64 = 350;

/// A mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    /// The primary button.
    Left,
    /// The secondary button.
    Right,
    /// Any other; the game has no use for it.
    Other,
}

impl Button {
    /// The button's bit in the game's button mask.
    const fn bit(self) -> u8 {
        match self {
            Button::Left => 1,
            Button::Right => 2,
            Button::Other => 0,
        }
    }
}

/// The open settings menu.
#[derive(Clone, Debug)]
struct Menu {
    /// The settings as the menu shows them, applied only by Apply.
    draft: DisplaySettings,
    selected: MenuItem,
    /// Opened at launch: Cancel quits, and the game warms up behind it.
    startup: bool,
    /// Why the last Apply failed.
    message: Option<&'static str>,
}

/// The desktop's state between input and the game.
#[derive(Clone, Debug)]
pub struct Controller {
    config: PathBuf,
    settings: DisplaySettings,
    menu: Option<Menu>,
    warmup: bool,
    held: Box<[bool; SCANCODES]>,
    buttons: u8,
    last_window: u8,
    paused: bool,
    quit: bool,
    inputs: Vec<InputKind>,
}

impl Controller {
    /// A controller saving to `config`, with the settings menu open at launch
    /// when `startup_menu` says so.
    pub fn new(config: PathBuf, settings: DisplaySettings, startup_menu: bool) -> Self {
        let last_window = if settings.window == FULLSCREEN { 1 } else { settings.window };
        let menu = startup_menu.then_some(Menu {
            draft: settings,
            selected: MenuItem::ScalingFilter,
            startup: true,
            message: None,
        });
        Controller {
            config,
            settings,
            menu,
            warmup: startup_menu,
            held: Box::new([false; SCANCODES]),
            buttons: 0,
            last_window,
            paused: false,
            quit: false,
            inputs: Vec::new(),
        }
    }

    /// The settings in force.
    pub fn settings(&self) -> &DisplaySettings {
        &self.settings
    }

    /// The settings the picture shows now: the menu's draft while it is open.
    pub fn preview(&self) -> &DisplaySettings {
        self.menu.as_ref().map_or(&self.settings, |menu| &menu.draft)
    }

    /// Whether the settings menu is open.
    pub fn menu_open(&self) -> bool {
        self.menu.is_some()
    }

    /// Whether the game is paused.
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Whether the player chose to quit.
    pub fn quit(&self) -> bool {
        self.quit
    }

    /// Game input produced since the last call, oldest first.
    pub fn take_inputs(&mut self) -> Vec<InputKind> {
        std::mem::take(&mut self.inputs)
    }

    /// Passes input straight to the game.
    pub fn send(&mut self, input: InputKind) {
        self.inputs.push(input);
    }

    /// A display frame begins, `elapsed_ms` into the session, with the game in
    /// BIOS video mode `video_mode`: the startup menu pauses the warmed-up game.
    pub fn tick(&mut self, elapsed_ms: u64, video_mode: u8) {
        if self.warmup && elapsed_ms >= WARMUP_MS && video_mode == VGA_MODE {
            self.paused = true;
            self.warmup = false;
        }
    }

    /// The window lost focus: nothing stays held.
    pub fn focus_lost(&mut self) {
        self.release_input();
    }

    fn release_input(&mut self) {
        self.held.fill(false);
        self.buttons = 0;
        self.inputs.push(InputKind::Clear);
    }

    /// The pointer moved to `at` in game pixels: inside the picture, or
    /// clamped to its nearest edge, which the game follows either way.
    pub fn pointer_moved(&mut self, at: (i32, i32)) {
        if self.menu.is_none() {
            self.inputs.push(InputKind::Mouse { x: at.0, y: at.1 });
        }
    }

    /// A mouse button went down or up with the pointer at `at`. Only a press
    /// inside the picture reaches the game; any release does.
    pub fn pointer(&mut self, at: (i32, i32), inside: bool, button: Option<(Button, bool)>) {
        if self.menu.is_some() {
            return;
        }
        if inside {
            self.inputs.push(InputKind::Mouse { x: at.0, y: at.1 });
        }
        if let Some((button, pressed)) = button.filter(|(button, _)| button.bit() != 0) {
            if pressed && inside {
                self.buttons |= button.bit();
            } else {
                self.buttons &= !button.bit();
            }
            self.inputs.push(InputKind::Buttons(self.buttons));
        }
    }

    /// The pointer is over menu item `hit` (on its left arrow when the flag is
    /// set), or over nothing: hovering selects, and a left press acts.
    pub fn menu_pointer(&mut self, hit: Option<(MenuItem, bool)>, left_press: bool) {
        let Some((item, on_left_arrow)) = hit else { return };
        self.select(item);
        if left_press {
            self.action(item, if on_left_arrow { -1 } else { 1 });
        }
    }

    /// A key went down (`pressed`) or up.
    pub fn key(&mut self, key: Key, pressed: bool) {
        if self.menu.is_some() {
            if pressed {
                self.menu_key(key);
            }
            return;
        }
        if pressed && key.keycode == keycode::F11 {
            if !key.repeat {
                self.open_menu();
            }
            return;
        }
        if let Some(held) =
            self.held.get_mut(usize::from(key.scancode)).filter(|_| key.scancode > 0)
        {
            *held = pressed;
        }
        if key.scancode == scancode::SPACE {
            self.inputs.push(InputKind::Space(pressed));
        }
        self.inputs.push(InputKind::Directions(keys::movement(&self.held)));
        if pressed {
            if key.keycode == keycode::RETURN && key.mods & ALT != 0 {
                if !key.repeat {
                    self.toggle_fullscreen();
                }
            } else {
                let event = keys::key_event(key);
                if event != 0 {
                    self.inputs.push(InputKind::Key(event));
                }
            }
        } else if keys::direction(key.scancode) != 0 || key.scancode == scancode::SPACE {
            self.inputs.push(InputKind::Release(u32::from(key.scancode)));
        }
    }

    fn toggle_fullscreen(&mut self) {
        self.settings.window =
            if self.settings.window == FULLSCREEN { self.last_window } else { FULLSCREEN };
        self.release_input();
    }

    fn open_menu(&mut self) {
        self.release_input();
        self.paused = true;
        self.warmup = false;
        self.menu = Some(Menu {
            draft: self.settings,
            selected: MenuItem::ScalingFilter,
            startup: false,
            message: None,
        });
    }

    fn close_menu(&mut self) {
        self.menu = None;
        self.warmup = false;
        self.release_input();
        self.paused = false;
    }

    fn menu_key(&mut self, key: Key) {
        let Some(current) = self.selected() else { return };
        let (selected, count) = (index(current), MenuItem::ALL.len());
        let on_row = selected < MenuItem::ROWS.len();
        match keys::menu_key(key) {
            keycode::ESCAPE => self.action(MenuItem::Cancel, 1),
            keycode::RETURN | keycode::KP_ENTER if !key.repeat => {
                self.action(if on_row { MenuItem::Apply } else { current }, 1);
            }
            keycode::SPACE if on_row || !key.repeat => self.action(current, 1),
            keycode::TAB => {
                let step = if key.mods & SHIFT != 0 { count - 1 } else { 1 };
                self.select(MenuItem::ALL[(selected + step) % count]);
            }
            keycode::UP => self.select(MenuItem::ALL[(selected + count - 1) % count]),
            keycode::DOWN => self.select(MenuItem::ALL[(selected + 1) % count]),
            keycode::LEFT if on_row => self.action(current, -1),
            keycode::LEFT if current == MenuItem::Comfort => self.select(MenuItem::Apply),
            keycode::LEFT => self.select(MenuItem::ALL[selected - 1]),
            keycode::RIGHT if on_row => self.action(current, 1),
            keycode::RIGHT if current == MenuItem::Apply => self.select(MenuItem::Comfort),
            keycode::RIGHT => self.select(MenuItem::ALL[selected + 1]),
            _ => {}
        }
    }

    fn select(&mut self, item: MenuItem) {
        if let Some(menu) = &mut self.menu {
            menu.selected = item;
        }
    }

    fn action(&mut self, item: MenuItem, direction: i32) {
        let Some(menu) = &mut self.menu else { return };
        match item {
            MenuItem::Comfort | MenuItem::Original => {
                let qol = menu.draft.qol;
                menu.draft = if item == MenuItem::Comfort {
                    menu::comfort(menu.draft.startup)
                } else {
                    menu::original(menu.draft.startup)
                };
                menu.draft.qol = qol;
            }
            MenuItem::Cancel if menu.startup => self.quit = true,
            MenuItem::Cancel => self.close_menu(),
            MenuItem::Apply => {
                if settings::save(&self.config, &menu.draft).is_err() {
                    menu.message =
                        Some("Could not save settings. Check that this game folder is writable.");
                    return;
                }
                self.settings = menu.draft;
                self.inputs.push(InputKind::Qol(self.settings.qol));
                if self.settings.window != FULLSCREEN {
                    self.last_window = self.settings.window;
                }
                self.close_menu();
            }
            row => {
                menu::change(&mut menu.draft, row, direction);
                menu.message = None;
            }
        }
    }

    /// Why the last Apply failed, while the menu shows it.
    pub fn message(&self) -> Option<&'static str> {
        self.menu.as_ref().and_then(|menu| menu.message)
    }

    /// The menu item under the cursor, while the menu is open.
    pub fn selected(&self) -> Option<MenuItem> {
        self.menu.as_ref().map(|menu| menu.selected)
    }

    /// Whether the menu opened at launch.
    pub fn startup_menu(&self) -> bool {
        self.menu.as_ref().is_some_and(|menu| menu.startup)
    }
}

fn index(item: MenuItem) -> usize {
    MenuItem::ALL.iter().position(|&candidate| candidate == item).expect("every item is listed")
}
