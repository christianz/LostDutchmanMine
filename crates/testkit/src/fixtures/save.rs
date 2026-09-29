//! Saved games as the original writes them: eight blocks of the data segment,
//! one after another, 6,066 bytes in all.

use std::path::Path;

use game::symbols::{Global, SAVE_BLOCKS, SAVE_SIZE};

use super::FixtureError;

/// A saved game, to read and change words of the data segment in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveGame {
    bytes: Vec<u8>,
}

impl SaveGame {
    /// Reads a saved game.
    ///
    /// # Errors
    ///
    /// When the file cannot be read or is not a complete saved game.
    pub fn read(path: &Path) -> Result<Self, FixtureError> {
        let bytes = std::fs::read(path).map_err(|source| FixtureError::io(path, source))?;
        if bytes.len() != SAVE_SIZE {
            return Err(FixtureError::Invalid(format!(
                "{}: {} bytes, a saved game has {SAVE_SIZE}",
                path.display(),
                bytes.len()
            )));
        }
        Ok(SaveGame { bytes })
    }

    /// Writes it.
    ///
    /// # Errors
    ///
    /// When the file cannot be written.
    pub fn write(&self, path: &Path) -> Result<(), FixtureError> {
        std::fs::write(path, &self.bytes).map_err(|source| FixtureError::io(path, source))
    }

    /// Where a word of the data segment is in the file, if it is saved.
    fn offset(global: Global) -> Option<usize> {
        let mut at = 0;
        for (start, length) in SAVE_BLOCKS {
            if (start.0..start.0 + length).contains(&global.0) && global.0 + 1 < start.0 + length {
                return Some(at + usize::from(global.0 - start.0));
            }
            at += usize::from(length);
        }
        None
    }

    /// A saved word.
    ///
    /// # Panics
    ///
    /// For a word no saved game holds: a fixture naming one is a bug.
    pub fn word(&self, global: Global) -> u16 {
        let at = Self::offset(global).expect("a saved word");
        u16::from_le_bytes([self.bytes[at], self.bytes[at + 1]])
    }

    /// Changes a saved word.
    ///
    /// # Panics
    ///
    /// For a word no saved game holds: a fixture naming one is a bug.
    pub fn set(&mut self, global: Global, value: u16) {
        let at = Self::offset(global).expect("a saved word");
        self.bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }
}
