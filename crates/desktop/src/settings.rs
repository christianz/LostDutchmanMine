//! The display settings, and the file that keeps them between runs.
//!
//! The file is a comment and then one `key=number` line per setting. Reading
//! it is forgiving: a missing file, an unknown key such as the retired `vsync`,
//! or a number that is not a valid choice leaves that setting as it was, so a
//! damaged file can never stop the game starting.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The [`DisplaySettings::window`] that means desktop fullscreen. The numbers
/// below it are the three window sizes, smallest first.
pub const FULLSCREEN: u8 = 3;

/// The fullscreen picture sizes, in percent, in the order the menu offers them.
pub const FULLSCREEN_SIZES: [u8; 3] = [100, 85, 70];

/// The brightness levels, in percent, darkest first.
pub const BRIGHTNESS_LEVELS: [u8; 5] = [80, 90, 100, 110, 120];

/// How the picture is enlarged to the display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scaling {
    /// Hard pixel edges, as the original.
    Crisp,
    /// Pixel edges softened as the picture is enlarged.
    Soft,
    /// Diagonals rounded by Scale2x before the enlargement.
    PixelArt,
}

/// The colour profile applied to the artwork.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Colour {
    /// The game's palette, unchanged.
    Original,
    /// Slightly more red and less blue.
    Warm,
    /// Stronger colours and contrast.
    Vivid,
    /// Muted colours and softer contrast.
    Gentle,
}

/// The CRT monitor effect's strength.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crt {
    /// A plain picture.
    Off,
    /// Light scanlines, phosphors and glow.
    Subtle,
    /// A bolder effect.
    Strong,
}

impl Scaling {
    /// Every filter, numbered in the file by its place here.
    pub const ALL: [Self; 3] = [Self::Crisp, Self::Soft, Self::PixelArt];
}

impl Colour {
    /// Every profile, numbered in the file by its place here.
    pub const ALL: [Self; 4] = [Self::Original, Self::Warm, Self::Vivid, Self::Gentle];
}

impl Crt {
    /// Every strength, numbered in the file by its place here.
    pub const ALL: [Self; 3] = [Self::Off, Self::Subtle, Self::Strong];
}

/// Everything the player chooses about the display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplaySettings {
    /// 0, 1 or 2 for a 960x720, 1280x960 or 1600x1200 window, or [`FULLSCREEN`].
    pub window: u8,
    /// The fullscreen picture as a percentage of the largest 4:3 area, one of
    /// [`FULLSCREEN_SIZES`]. Windows always use the whole window.
    pub size: u8,
    /// How the picture is enlarged.
    pub scaling: Scaling,
    /// The colour profile.
    pub colour: Colour,
    /// The CRT monitor effect.
    pub crt: Crt,
    /// Picture brightness in percent, one of [`BRIGHTNESS_LEVELS`].
    pub brightness: u8,
    /// Whether the settings menu opens at launch.
    pub startup: bool,
    /// Whether the optional quality-of-life improvements are on.
    pub qol: bool,
}

impl Default for DisplaySettings {
    /// A 1280x960 window of soft pixels in the original colours, with the
    /// improvements on and the menu shown at launch.
    fn default() -> Self {
        Self {
            window: 1,
            size: 100,
            scaling: Scaling::Soft,
            colour: Colour::Original,
            crt: Crt::Off,
            brightness: 100,
            startup: true,
            qol: true,
        }
    }
}

/// The two settings that are switches: 0 is off, 1 is on.
const SWITCH: [bool; 2] = [false, true];

/// The comment that opens the file, for anyone who finds it.
const HEADER: &str = "# Lost Dutchman Mine display settings. F11 opens the settings window.";

/// The settings in `file`, with defaults for anything it lacks or gets wrong.
pub fn load(file: impl AsRef<Path>) -> DisplaySettings {
    let mut settings = DisplaySettings::default();
    // A file that cannot be read reads as empty: the first launch has none.
    let text = fs::read(file).unwrap_or_default();
    for (key, number) in text.split(|&byte| byte == b'\n').filter_map(setting) {
        settings.set(key, number);
    }
    settings
}

/// Writes `settings` to `file`, creating its folder if need be.
///
/// The settings go to a temporary file beside it that then replaces it, so a
/// failed save leaves the previous settings intact.
///
/// # Errors
///
/// Any error creating the folder, writing the temporary file or replacing `file`.
pub fn save(file: impl AsRef<Path>, settings: &DisplaySettings) -> io::Result<()> {
    let file = file.as_ref();
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder)?;
    }
    let temporary = beside(file, ".tmp");
    fs::write(&temporary, text(*settings))?;
    fs::rename(&temporary, file)
}

impl DisplaySettings {
    /// Takes one line's number for `key`, if it is a valid choice there.
    fn set(&mut self, key: &[u8], number: i32) {
        let byte = u8::try_from(number).ok();
        match key {
            b"window" => {
                self.window = byte.filter(|&window| window <= FULLSCREEN).unwrap_or(self.window);
            }
            b"size" => {
                self.size =
                    byte.filter(|size| FULLSCREEN_SIZES.contains(size)).unwrap_or(self.size);
            }
            b"scaling" => self.scaling = numbered(&Scaling::ALL, number).unwrap_or(self.scaling),
            b"colour" => self.colour = numbered(&Colour::ALL, number).unwrap_or(self.colour),
            b"crt" => self.crt = numbered(&Crt::ALL, number).unwrap_or(self.crt),
            b"brightness" => {
                self.brightness = byte
                    .filter(|level| BRIGHTNESS_LEVELS.contains(level))
                    .unwrap_or(self.brightness);
            }
            b"startup" => self.startup = numbered(&SWITCH, number).unwrap_or(self.startup),
            b"qol" => self.qol = numbered(&SWITCH, number).unwrap_or(self.qol),
            // Older files may still name retired settings such as `vsync`.
            _ => {}
        }
    }
}

/// A `key=number` line's key and number; `None` for comments and malformed lines.
fn setting(line: &[u8]) -> Option<(&[u8], i32)> {
    let equals = line.iter().position(|&byte| byte == b'=')?;
    Some((&line[..equals], integer(&line[equals + 1..])?))
}

/// A whole decimal number with an optional sign, perhaps padded with
/// whitespace, which covers Windows line endings. These are exactly the values
/// the C++ build read, so each build reads the other's files alike.
fn integer(value: &[u8]) -> Option<i32> {
    std::str::from_utf8(value).ok()?.trim_matches(is_c_space).parse().ok()
}

/// Whether C's `isspace` counts `c` as whitespace; unlike Rust's ASCII
/// whitespace, that includes the vertical tab.
fn is_c_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\x0B' | '\x0C' | '\r')
}

/// The choice `number` names in the file, where choices count from 0.
fn numbered<T: Copy>(choices: &[T], number: i32) -> Option<T> {
    choices.get(usize::try_from(number).ok()?).copied()
}

/// The file's text: the comment, then each setting in a fixed order.
fn text(settings: DisplaySettings) -> String {
    let DisplaySettings { window, size, scaling, colour, crt, brightness, startup, qol } = settings;
    format!(
        "{HEADER}\nwindow={window}\nsize={size}\nscaling={}\ncolour={}\ncrt={}\n\
         brightness={brightness}\nstartup={}\nqol={}\n",
        scaling as u8,
        colour as u8,
        crt as u8,
        u8::from(startup),
        u8::from(qol),
    )
}

/// `file` with `suffix` added to its name.
fn beside(file: &Path, suffix: &str) -> PathBuf {
    let mut name = OsString::from(file);
    name.push(suffix);
    name.into()
}
