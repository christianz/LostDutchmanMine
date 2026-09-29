//! Starting the game from the player's own copy.

use std::path::PathBuf;

use dos::DosError;
use machine::{Address, Machine};
use translate::image::ImageError;

use crate::{Game, translated};

/// The executable in the game folder.
const EXECUTABLE: &[u8] = b"LDM.EXE";

/// Why the game cannot start.
#[derive(Debug, thiserror::Error)]
pub enum BootError {
    /// This build translated no executable.
    #[error("this build has no translated game: rebuild with LDM_EXE=/path/to/LDM.EXE")]
    NotTranslated,
    /// The game folder has no readable LDM.EXE.
    #[error("{path}: {source}")]
    Read {
        /// Where it was looked for.
        path: PathBuf,
        /// The failure.
        source: std::io::Error,
    },
    /// The executable is not the release this build translated.
    #[error(transparent)]
    Image(#[from] ImageError),
    /// DOS could not load it.
    #[error(transparent)]
    Dos(#[from] DosError),
}

/// Loads LDM.EXE from `data` into a fresh machine, as DOS would start it, with
/// saves going to `saves`.
///
/// # Errors
///
/// [`BootError`] when the build has no translation, or the game folder does not
/// hold the release it translated.
pub fn boot(data: PathBuf, saves: PathBuf, qol: bool) -> Result<(Machine, Game), BootError> {
    if translated::SOURCE.is_none() {
        return Err(BootError::NotTranslated);
    }
    let mut game = Game::new(data, saves, qol);
    let path = game.dos.files.resolve(EXECUTABLE, false)?;
    let exe = std::fs::read(&path).map_err(|source| BootError::Read { path, source })?;
    let image = translate::image::load(&exe)?;
    let mut m = Machine::new();
    let stack = Address::new(image.stack_segment, image.stack_pointer);
    let program = dos::Image {
        bytes: &image.bytes,
        relocations: &image.relocations,
        entry: image.entry,
        stack,
    };
    dos::boot(&mut m, &program)?;
    // The text renderer's 256 eight-row glyphs, before relocation touches them.
    game.overlay.load_font(&image.bytes);
    Ok((m, game))
}
