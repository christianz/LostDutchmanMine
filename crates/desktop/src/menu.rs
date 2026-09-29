//! The settings menu's content, and how its rows change the settings.
//!
//! The menu opens at launch and again with F11 during play. Its first seven
//! items are rows adjusted with the arrows; the rest are buttons, named for
//! where the menu opened: Quit and Play at launch, Cancel and Apply during play.

use crate::settings::{
    BRIGHTNESS_LEVELS, Colour, Crt, DisplaySettings, FULLSCREEN, FULLSCREEN_SIZES, Scaling,
};

/// One row or button of the settings menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuItem {
    /// The window size, or fullscreen at a picture size.
    Display,
    /// How the picture is enlarged.
    ScalingFilter,
    /// The CRT monitor effect.
    CrtMonitor,
    /// The colour profile.
    ColourProfile,
    /// Picture brightness.
    Brightness,
    /// Whether the menu opens at launch.
    Startup,
    /// The optional quality-of-life improvements.
    QualityOfLife,
    /// A preset for large monitors: [`comfort`].
    Comfort,
    /// A preset close to the original game: [`original`].
    Original,
    /// Leaves the menu without keeping the changes.
    Cancel,
    /// Keeps the changes and plays.
    Apply,
}

impl MenuItem {
    /// The rows, top to bottom.
    pub const ROWS: [Self; 7] = [
        Self::Display,
        Self::ScalingFilter,
        Self::CrtMonitor,
        Self::ColourProfile,
        Self::Brightness,
        Self::Startup,
        Self::QualityOfLife,
    ];

    /// Every item: the rows, the two presets, then Cancel and Apply.
    pub const ALL: [Self; 11] = [
        Self::Display,
        Self::ScalingFilter,
        Self::CrtMonitor,
        Self::ColourProfile,
        Self::Brightness,
        Self::Startup,
        Self::QualityOfLife,
        Self::Comfort,
        Self::Original,
        Self::Cancel,
        Self::Apply,
    ];
}

/// The Display row's choices: the three windows, then fullscreen at each of
/// [`FULLSCREEN_SIZES`].
const SCREENS: [&str; 6] = [
    "960 x 720 window",
    "1280 x 960 window",
    "1600 x 1200 window",
    "Fullscreen",
    "Fullscreen 85%",
    "Fullscreen 70%",
];

/// The difference between neighbouring [`BRIGHTNESS_LEVELS`], in percent.
const BRIGHTNESS_STEP: u8 = 10;

/// The picture percentage of a window, which always fills it.
const WHOLE_PICTURE: u8 = 100;

/// The item's name: a row's label, or the text on a button.
pub fn label(item: MenuItem, startup: bool) -> &'static str {
    match item {
        MenuItem::Display => "Display",
        MenuItem::ScalingFilter => "Scaling",
        MenuItem::CrtMonitor => "CRT monitor",
        MenuItem::ColourProfile => "Colour",
        MenuItem::Brightness => "Brightness",
        MenuItem::Startup => "Show at startup",
        MenuItem::QualityOfLife => "QoL improvements",
        MenuItem::Comfort => "Comfort",
        MenuItem::Original => "Original look",
        MenuItem::Cancel if startup => "Quit",
        MenuItem::Cancel => "Cancel",
        MenuItem::Apply if startup => "Play",
        MenuItem::Apply => "Apply & resume",
    }
}

/// What a row shows between its arrows; empty for a button.
pub fn value(settings: &DisplaySettings, row: MenuItem) -> String {
    let value = match row {
        MenuItem::Display => SCREENS[screen(*settings)],
        MenuItem::ScalingFilter => match settings.scaling {
            Scaling::Crisp => "Crisp pixels",
            Scaling::Soft => "Soft pixels",
            Scaling::PixelArt => "Pixel art",
        },
        MenuItem::CrtMonitor => match settings.crt {
            Crt::Off => "Off",
            Crt::Subtle => "Subtle",
            Crt::Strong => "Strong",
        },
        MenuItem::ColourProfile => match settings.colour {
            Colour::Original => "Original",
            Colour::Warm => "Warm",
            Colour::Vivid => "Vivid",
            Colour::Gentle => "Gentle",
        },
        MenuItem::Brightness => return format!("{}%", settings.brightness),
        MenuItem::Startup if settings.startup => "Yes",
        MenuItem::Startup => "No",
        MenuItem::QualityOfLife if settings.qol => "On",
        MenuItem::QualityOfLife => "Off",
        MenuItem::Comfort | MenuItem::Original | MenuItem::Cancel | MenuItem::Apply => "",
    };
    value.to_owned()
}

/// The two lines of help shown under the preview while `item` is selected.
pub fn help(item: MenuItem, startup: bool) -> [&'static str; 2] {
    match item {
        MenuItem::Display => [
            "Fullscreen keeps the 4:3 shape at any resolution.",
            "85% or 70% leaves a border on large monitors.",
        ],
        MenuItem::ScalingFilter => [
            "Soft blends edges. Pixel art rounds diagonals.",
            "Crisp keeps the original hard pixel edges.",
        ],
        MenuItem::CrtMonitor => [
            "Scanlines, phosphor texture and a gentle glow.",
            "Subtle is light. Strong gives a bolder effect.",
        ],
        MenuItem::ColourProfile => [
            "Adjust the colour of the original artwork.",
            "Original keeps the game's palette unchanged.",
        ],
        MenuItem::Brightness => {
            ["Adjust picture brightness to suit your room.", "100% keeps the original brightness."]
        }
        MenuItem::Startup => {
            ["Choose whether this menu opens at launch.", "You can always open it again with F11."]
        }
        MenuItem::QualityOfLife => [
            "Toolbar labels, arrow pointer, mouse aiming,",
            "faster walking; mules unlock one at a time.",
        ],
        MenuItem::Comfort => [
            "Fullscreen 85%, soft pixels, gentle colours,",
            "CRT off and 100% brightness. Adjust to taste.",
        ],
        MenuItem::Original => [
            "Crisp pixels, original colours, CRT off,",
            "in a 960 x 720 window with the 4:3 shape.",
        ],
        MenuItem::Cancel if startup => {
            ["Quit closes the game without saving settings.", "Your saved games are not affected."]
        }
        MenuItem::Cancel => {
            ["Cancel discards these changes and resumes.", "F11 opens this menu again during play."]
        }
        MenuItem::Apply if startup => [
            "Play saves these settings and starts the game.",
            "F11 opens this menu again during play.",
        ],
        MenuItem::Apply => [
            "Apply saves these settings and resumes play.",
            "F11 opens this menu again during play.",
        ],
    }
}

/// The key hint along the bottom of the menu.
pub fn keys(startup: bool) -> &'static str {
    if startup {
        "Arrows adjust   Enter confirms   Esc quits"
    } else {
        "Arrows adjust   Enter confirms   Esc cancels"
    }
}

/// Steps `row`'s setting by `direction`, -1 for left and 1 for right, wrapping
/// round at either end. Buttons change nothing.
pub fn change(settings: &mut DisplaySettings, row: MenuItem, direction: i32) {
    match row {
        MenuItem::Display => change_screen(settings, direction),
        MenuItem::ScalingFilter => {
            settings.scaling = step(&Scaling::ALL, settings.scaling as usize, direction);
        }
        MenuItem::CrtMonitor => settings.crt = step(&Crt::ALL, settings.crt as usize, direction),
        MenuItem::ColourProfile => {
            settings.colour = step(&Colour::ALL, settings.colour as usize, direction);
        }
        MenuItem::Brightness => {
            let level = settings.brightness.saturating_sub(BRIGHTNESS_LEVELS[0]) / BRIGHTNESS_STEP;
            settings.brightness = step(&BRIGHTNESS_LEVELS, level.into(), direction);
        }
        MenuItem::Startup => settings.startup = !settings.startup,
        MenuItem::QualityOfLife => settings.qol = !settings.qol,
        MenuItem::Comfort | MenuItem::Original | MenuItem::Cancel | MenuItem::Apply => {}
    }
}

/// The Comfort preset: fullscreen at 85%, soft pixels, gentle colours, CRT off
/// and 100% brightness.
pub fn comfort(startup: bool) -> DisplaySettings {
    DisplaySettings {
        window: FULLSCREEN,
        size: 85,
        colour: Colour::Gentle,
        startup,
        ..DisplaySettings::default()
    }
}

/// The Original look preset: crisp pixels in the original colours with CRT
/// off, in the 960x720 window.
pub fn original(startup: bool) -> DisplaySettings {
    DisplaySettings { window: 0, scaling: Scaling::Crisp, startup, ..DisplaySettings::default() }
}

/// How much of the largest 4:3 area the picture fills, in percent: the chosen
/// size in fullscreen, and all of it in a window.
pub fn picture_percent(settings: &DisplaySettings) -> u8 {
    if settings.window == FULLSCREEN { settings.size } else { WHOLE_PICTURE }
}

/// The Display row's current choice, as an index into [`SCREENS`].
fn screen(settings: DisplaySettings) -> usize {
    if settings.window < FULLSCREEN {
        return settings.window.into();
    }
    // A size the menu does not offer shows as the first, full size.
    let offered = FULLSCREEN_SIZES.iter().position(|&size| size == settings.size);
    usize::from(FULLSCREEN) + offered.unwrap_or(0)
}

/// Steps the Display row, where fullscreen at each picture size follows the
/// largest window.
fn change_screen(settings: &mut DisplaySettings, direction: i32) {
    let choice = cycle(screen(*settings), SCREENS.len(), direction);
    match choice.checked_sub(FULLSCREEN.into()) {
        Some(offered) => {
            settings.window = FULLSCREEN;
            settings.size = FULLSCREEN_SIZES[offered];
        }
        // A window keeps the fullscreen size, so switching back to fullscreen
        // restores the picture size chosen for it.
        None => settings.window = choice as u8,
    }
}

/// The choice `direction` steps to from `choices[index]`, wrapping round.
fn step<T: Copy>(choices: &[T], index: usize, direction: i32) -> T {
    choices[cycle(index, choices.len(), direction)]
}

/// The index `direction` steps to from `index` among `count` choices, wrapping
/// round at either end.
fn cycle(index: usize, count: usize, direction: i32) -> usize {
    (index + direction.rem_euclid(count as i32) as usize) % count
}
