//! The settings menu: its rows, their values, the presets and the wording.

use desktop::font::text_width;
use desktop::menu::{self, MenuItem, picture_percent};
use desktop::settings::{Colour, Crt, DisplaySettings, Scaling};

/// Help lines sit under the 432-unit-wide live preview, at size 17.
const HELP_WIDTH: f32 = 432.0;
/// Row labels end before the "<" arrow, at size 18.
const LABEL_WIDTH: f32 = 205.0;
/// Values sit between the "<" and ">" arrows, at size 17.
const VALUE_WIDTH: f32 = 190.0;
/// The Comfort, Original look, Cancel and Apply buttons' widths.
const BUTTON_WIDTHS: [f32; 4] = [240.0, 240.0, 132.0, 200.0];
/// Button labels keep this much clear of the button's edges, in total.
const BUTTON_PADDING: f32 = 16.0;
/// The key hint ends before the Apply button, at size 15.
const KEYS_WIDTH: f32 = 580.0;

fn help(item: MenuItem, startup: bool) -> String {
    menu::help(item, startup).join(" ")
}

/// The first `count` values a row shows as it is stepped right.
fn values(row: MenuItem, count: usize) -> Vec<String> {
    let mut settings = DisplaySettings::default();
    (0..count)
        .map(|_| {
            let value = menu::value(&settings, row);
            menu::change(&mut settings, row, 1);
            value
        })
        .collect()
}

#[test]
fn the_menu_has_seven_rows_then_four_buttons() {
    let labels: Vec<_> = MenuItem::ROWS.iter().map(|&row| menu::label(row, true)).collect();
    assert_eq!(
        labels,
        [
            "Display",
            "Scaling",
            "CRT monitor",
            "Colour",
            "Brightness",
            "Show at startup",
            "QoL improvements"
        ]
    );
    assert_eq!(MenuItem::ALL[..7], MenuItem::ROWS);
    assert_eq!(
        MenuItem::ALL[7..],
        [MenuItem::Comfort, MenuItem::Original, MenuItem::Cancel, MenuItem::Apply]
    );
}

#[test]
fn display_cycles_through_windows_then_fullscreen_sizes() {
    assert_eq!(
        values(MenuItem::Display, 7),
        [
            "1280 x 960 window",
            "1600 x 1200 window",
            "Fullscreen",
            "Fullscreen 85%",
            "Fullscreen 70%",
            "960 x 720 window",
            "1280 x 960 window"
        ]
    );
}

#[test]
fn left_from_the_smallest_window_selects_fullscreen_70() {
    let mut settings = DisplaySettings { window: 0, ..DisplaySettings::default() };
    menu::change(&mut settings, MenuItem::Display, -1);
    assert_eq!((settings.window, settings.size), (3, 70));
    assert_eq!(picture_percent(&settings), 70);
}

#[test]
fn windows_always_use_the_whole_window() {
    let settings = DisplaySettings { window: 1, size: 85, ..DisplaySettings::default() };
    assert_eq!(picture_percent(&settings), 100);
}

#[test]
fn fullscreen_keeps_its_picture_size() {
    let settings = DisplaySettings { window: 3, size: 85, ..DisplaySettings::default() };
    assert_eq!(picture_percent(&settings), 85);
}

#[test]
fn crt_strengths_are_not_named_like_soft_pixels() {
    assert_eq!(values(MenuItem::CrtMonitor, 4), ["Off", "Subtle", "Strong", "Off"]);
}

#[test]
fn every_row_wraps_both_ways() {
    for row in MenuItem::ROWS {
        let mut settings = DisplaySettings::default();
        menu::change(&mut settings, row, 1);
        menu::change(&mut settings, row, -1);
        assert_eq!(settings, DisplaySettings::default(), "{row:?}: right then left");
    }
    assert_eq!(
        values(MenuItem::ScalingFilter, 4),
        ["Soft pixels", "Pixel art", "Crisp pixels", "Soft pixels"]
    );
    assert_eq!(
        values(MenuItem::ColourProfile, 5),
        ["Original", "Warm", "Vivid", "Gentle", "Original"]
    );
    assert_eq!(values(MenuItem::Brightness, 6), ["100%", "110%", "120%", "80%", "90%", "100%"]);
    assert_eq!(values(MenuItem::Startup, 3), ["Yes", "No", "Yes"]);
}

#[test]
fn qol_is_an_on_off_toggle() {
    assert_eq!(values(MenuItem::QualityOfLife, 3), ["On", "Off", "On"]);
}

#[test]
fn buttons_have_no_value_and_change_nothing() {
    for button in [MenuItem::Comfort, MenuItem::Original, MenuItem::Cancel, MenuItem::Apply] {
        let mut settings = DisplaySettings::default();
        menu::change(&mut settings, button, 1);
        assert_eq!(settings, DisplaySettings::default(), "{button:?}");
        assert_eq!(menu::value(&settings, button), "", "{button:?}");
    }
}

#[test]
fn buttons_are_named_for_where_the_menu_opened() {
    assert_eq!(menu::label(MenuItem::Comfort, true), "Comfort", "the preset is not specific to 4K");
    assert_eq!(menu::label(MenuItem::Original, true), "Original look");
    assert_eq!(menu::label(MenuItem::Cancel, true), "Quit");
    assert_eq!(menu::label(MenuItem::Cancel, false), "Cancel");
    assert_eq!(menu::label(MenuItem::Apply, true), "Play");
    assert_eq!(menu::label(MenuItem::Apply, false), "Apply & resume");
}

#[test]
fn help_explains_each_item() {
    let quality = help(MenuItem::QualityOfLife, false);
    assert!(
        quality.contains("mule") && quality.contains("walking"),
        "QoL names its gameplay changes"
    );
    let crt = help(MenuItem::CrtMonitor, false);
    assert!(!crt.contains("Soft") && !crt.contains("Classic"), "CRT help uses the strength names");
    assert!(help(MenuItem::Cancel, true).contains("Quit"));
    assert!(help(MenuItem::Cancel, false).contains("Cancel"));
    assert!(help(MenuItem::Apply, true).contains("Play"));
    assert!(help(MenuItem::Apply, false).contains("Apply"));
}

#[test]
fn the_key_hint_names_what_esc_does() {
    assert!(menu::keys(true).contains("Esc quits"));
    assert!(menu::keys(false).contains("Esc cancels"));
    for startup in [true, false] {
        assert!(!menu::keys(startup).contains("F11"), "F11 does nothing inside the menu");
    }
}

#[test]
fn the_comfort_preset_is_fullscreen_85_with_gentle_colours() {
    let comfort = menu::comfort(true);
    let expected = DisplaySettings {
        window: 3,
        size: 85,
        scaling: Scaling::Soft,
        colour: Colour::Gentle,
        crt: Crt::Off,
        brightness: 100,
        startup: true,
        qol: true,
    };
    assert_eq!(comfort, expected);
    assert_eq!(menu::value(&comfort, MenuItem::Display), "Fullscreen 85%");
    assert!(!menu::comfort(false).startup);
}

#[test]
fn the_original_look_is_crisp_pixels_in_the_smallest_window() {
    let original = menu::original(false);
    assert_eq!(original.window, 0);
    assert_eq!(picture_percent(&original), 100);
    assert_eq!(original.scaling, Scaling::Crisp);
    assert_eq!(original.colour, Colour::Original);
    assert_eq!(original.crt, Crt::Off);
    assert!(!original.startup);
    assert!(menu::original(true).startup);
}

#[test]
fn every_label_value_help_line_and_key_hint_fits_its_box() {
    for startup in [true, false] {
        for item in MenuItem::ALL {
            for line in menu::help(item, startup) {
                assert!(
                    text_width(line, 17.0) <= HELP_WIDTH,
                    "help fits under the preview: {line}"
                );
            }
        }
        for row in MenuItem::ROWS {
            let label = menu::label(row, startup);
            assert!(text_width(label, 18.0) <= LABEL_WIDTH, "row label fits: {label}");
        }
        for (&button, width) in MenuItem::ALL[7..].iter().zip(BUTTON_WIDTHS) {
            let label = menu::label(button, startup);
            assert!(
                text_width(label, 18.0) <= width - BUTTON_PADDING,
                "button label fits: {label}"
            );
        }
        assert!(text_width(menu::keys(startup), 15.0) <= KEYS_WIDTH, "key hint fits");
    }
    let mut settings = DisplaySettings::default();
    for row in MenuItem::ROWS {
        for _ in 0..6 {
            let value = menu::value(&settings, row);
            assert!(
                text_width(&value, 17.0) <= VALUE_WIDTH,
                "value fits between the arrows: {value}"
            );
            menu::change(&mut settings, row, 1);
        }
    }
}
