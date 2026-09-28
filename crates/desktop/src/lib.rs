//! The desktop front end's display pipeline, free of any windowing library.
//!
//! It owns the display settings file, the settings menu's rows and wording, and
//! every step that turns the game's 320x200 frame into the picture on screen:
//! placing it at 4:3, colour grading, enlarging and the CRT effect. The window
//! and renderer that show the result live elsewhere, and nothing here changes
//! the game's palette, pixels or timing.

pub mod font;
pub mod menu;
pub mod pixels;
pub mod settings;
