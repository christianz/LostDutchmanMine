//! The DOS and BIOS services Lost Dutchman Mine uses.
//!
//! Only what the game calls is implemented, each exactly as the original native
//! build did; anything else stops with a diagnostic rather than a guess. Files
//! are read from the player's game folder and written to a separate saves folder.

mod boot;
mod clock;
mod files;
mod keyboard;
mod mouse;
mod services;
mod video;

use std::path::PathBuf;

pub use boot::{Image, boot};
pub use clock::Clock;
pub use files::Files;
pub use keyboard::{
    KEY_MOVEMENT_SHIFT, KEY_REPEAT, KEY_SOURCE_SHIFT, Keyboard, bios_key, direction_scan,
    repeat_from,
};
pub use mouse::{Mouse, MouseInput, MouseSample};
pub use video::Video;

/// Why a service could not run.
#[derive(Debug, thiserror::Error)]
pub enum DosError {
    /// A service the game was never seen to call.
    #[error("unimplemented service INT {number:02x}h with AX={ax:04x}")]
    Unsupported {
        /// The interrupt.
        number: u8,
        /// AX at the call.
        ax: u16,
    },
    /// A string argument with no terminator within its segment.
    #[error("unterminated string at {segment:04x}:{offset:04x}")]
    Unterminated {
        /// Its segment.
        segment: u16,
        /// Its offset.
        offset: u16,
    },
    /// A file name that climbs out of the game's folders.
    #[error("the game asked for a path outside its folders: {0}")]
    ParentPath(String),
    /// A host file operation failed.
    #[error("{path}: {source}")]
    Io {
        /// The file.
        path: PathBuf,
        /// The failure.
        source: std::io::Error,
    },
    /// The program image does not fit below the video memory.
    #[error("the program image is too large: {0} bytes")]
    ImageTooLarge(usize),
}

/// The DOS and BIOS state of one session.
#[derive(Debug)]
pub struct Dos {
    /// False once the program has exited.
    pub running: bool,
    /// True while the program waits for input it has not received.
    pub waiting: bool,
    /// The BIOS keyboard buffer.
    pub keyboard: Keyboard,
    /// The mouse driver.
    pub mouse: Mouse,
    /// The video BIOS.
    pub video: Video,
    /// Date, time and the BIOS tick count.
    pub clock: Clock,
    /// Open files and the folders they come from.
    pub files: Files,
    /// Whether INT 33h may park the pointer (classic behaviour, QoL off).
    pub park_cursor: bool,
    /// Text the program printed through DOS or the BIOS.
    pub console: Vec<u8>,
    /// The next free paragraph for DOS memory allocation.
    next_paragraph: u16,
}

impl Dos {
    /// A session reading from `data` and writing to `saves`.
    pub fn new(data: PathBuf, saves: PathBuf) -> Self {
        Dos {
            running: true,
            waiting: false,
            keyboard: Keyboard::default(),
            mouse: Mouse::default(),
            video: Video::default(),
            clock: Clock::default(),
            files: Files::new(data, saves),
            park_cursor: false,
            console: Vec::new(),
            next_paragraph: 0x9000,
        }
    }
}

impl Default for Dos {
    fn default() -> Self {
        Dos::new(PathBuf::from("."), PathBuf::from("saves"))
    }
}
