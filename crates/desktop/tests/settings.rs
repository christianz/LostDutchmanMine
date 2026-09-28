//! The display settings file: defaults, round trips and forgiving reads.

use std::fs;
use std::path::PathBuf;

use desktop::menu::{self, MenuItem, picture_percent};
use desktop::settings::{self, Colour, Crt, DisplaySettings, Scaling};

/// A folder of its own for one test's settings file, removed afterwards.
struct Scratch {
    folder: PathBuf,
}

impl Scratch {
    fn new(test: &str) -> Self {
        let folder =
            std::env::temp_dir().join(format!("ldm-display-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("the scratch folder can be created");
        Self { folder }
    }

    fn ini(&self) -> PathBuf {
        self.folder.join("display.ini")
    }

    fn write(&self, text: &str) -> PathBuf {
        fs::write(self.ini(), text).expect("the settings file can be written");
        self.ini()
    }

    fn saved(&self, settings: DisplaySettings) -> String {
        settings::save(self.ini(), &settings).expect("the settings can be saved");
        fs::read_to_string(self.ini()).expect("the saved settings can be read")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.folder);
    }
}

fn chosen() -> DisplaySettings {
    DisplaySettings {
        window: 3,
        size: 85,
        scaling: Scaling::PixelArt,
        colour: Colour::Warm,
        crt: Crt::Strong,
        brightness: 110,
        startup: false,
        qol: false,
    }
}

#[test]
fn defaults_are_a_1280_window_of_soft_original_pixels() {
    let defaults = DisplaySettings {
        window: 1,
        size: 100,
        scaling: Scaling::Soft,
        colour: Colour::Original,
        crt: Crt::Off,
        brightness: 100,
        startup: true,
        qol: true,
    };
    assert_eq!(DisplaySettings::default(), defaults);
}

#[test]
fn first_launch_opens_the_menu_with_crt_off_and_improvements_on() {
    let scratch = Scratch::new("first-launch");
    let settings = settings::load(scratch.ini());
    assert!(settings.startup, "first launch shows the settings");
    assert_eq!(settings.crt, Crt::Off, "existing configurations keep CRT disabled");
    assert!(settings.qol, "new and existing configurations enable optional improvements");
}

#[test]
fn settings_round_trip_including_disabled_improvements() {
    let scratch = Scratch::new("round-trip");
    settings::save(scratch.ini(), &chosen()).expect("the settings can be saved");
    assert_eq!(settings::load(scratch.ini()), chosen());
}

#[test]
fn saving_replaces_existing_settings() {
    let scratch = Scratch::new("replace");
    settings::save(scratch.ini(), &chosen()).expect("the first save succeeds");
    let replaced = DisplaySettings { window: 0, colour: Colour::Original, ..chosen() };
    settings::save(scratch.ini(), &replaced).expect("the second save succeeds");
    assert_eq!(settings::load(scratch.ini()), replaced);
}

#[test]
fn every_setting_the_menu_offers_survives_a_save() {
    let scratch = Scratch::new("every-setting");
    let mut offered = DisplaySettings::default();
    for row in MenuItem::ROWS {
        for _ in 0..6 {
            menu::change(&mut offered, row, 1);
            settings::save(scratch.ini(), &offered).expect("the settings can be saved");
            assert_eq!(settings::load(scratch.ini()), offered, "{row:?}");
        }
    }
}

#[test]
fn the_file_keeps_its_format_and_key_order() {
    let scratch = Scratch::new("format");
    let expected = "# Lost Dutchman Mine display settings. F11 opens the settings window.\n\
                    window=3\nsize=85\nscaling=2\ncolour=1\ncrt=2\nbrightness=110\nstartup=0\nqol=0\n";
    assert_eq!(scratch.saved(chosen()), expected);
}

#[test]
fn saving_leaves_no_temporary_file_behind() {
    let scratch = Scratch::new("no-temporary");
    scratch.saved(chosen());
    assert!(!scratch.folder.join("display.ini.tmp").exists());
}

#[test]
fn saving_creates_missing_folders() {
    let scratch = Scratch::new("folders");
    let nested = scratch.folder.join("config").join("ldm").join("display.ini");
    settings::save(&nested, &chosen()).expect("the settings can be saved");
    assert_eq!(settings::load(&nested), chosen());
}

#[test]
fn malformed_settings_fall_back_to_defaults() {
    let scratch = Scratch::new("malformed");
    let ini = scratch.write(
        "window=999\nsize=-100\nscaling=4294967296\ncolour=garbage\ncrt=-1\nbrightness=+\n\
         vsync=2\nstartup=0junk\nqol=2\n",
    );
    assert_eq!(settings::load(ini), DisplaySettings::default());
}

#[test]
fn old_windowed_picture_sizes_load_and_are_ignored() {
    let scratch = Scratch::new("legacy");
    let legacy = settings::load(scratch.write("window=1\nsize=85\nvsync=0\n"));
    assert_eq!(legacy.window, 1);
    assert_eq!(picture_percent(&legacy), 100);
}

#[test]
fn vsync_is_no_longer_a_setting() {
    let scratch = Scratch::new("vsync");
    let legacy = settings::load(scratch.write("window=1\nsize=85\nvsync=0\n"));
    assert!(!scratch.saved(legacy).contains("vsync"));
}

#[test]
fn values_may_be_padded_with_whitespace_but_keys_may_not() {
    let scratch = Scratch::new("whitespace");
    let padded = settings::load(scratch.write("window= 2\r\n size=70\ncrt=\t1 \n"));
    assert_eq!(padded.window, 2, "Windows line endings and spaces around values are fine");
    assert_eq!(padded.size, 100, "a key with a space before it is a different key");
    assert_eq!(padded.crt, Crt::Subtle);
}

#[test]
fn a_later_valid_line_overrides_an_earlier_one() {
    let scratch = Scratch::new("override");
    let loaded = settings::load(scratch.write("window=2\nwindow=0\nwindow=7\nbrightness=90\n"));
    assert_eq!(loaded.window, 0, "an invalid line keeps the value before it");
    assert_eq!(loaded.brightness, 90);
}
