//! The saved game a harness would write, and whether it matches a fixture.
//!
//! A fixture is the eight blocks of the data segment the original save
//! routine writes, as `LDMSAVE1.SAV`, beside the save index `LDMSAVE.LDM` the
//! game would read. `cargo xtask fixtures` builds them from the moments in
//! `crate::moments`; these checks confirm the tests still reach them.

use std::path::PathBuf;

use game::symbols::SAVE_BLOCKS;

use super::{Harness, workspace};

/// A fixture's saved game.
const SAVED_GAME: &str = "LDMSAVE1.SAV";
/// The save index beside it.
const SAVE_INDEX: &str = "LDMSAVE.LDM";

/// The fixtures: `.local/fixtures` in the workspace.
fn fixtures() -> PathBuf {
    workspace().join(".local/fixtures")
}

/// The data-segment address of byte `at` of a saved game.
fn address(mut at: usize) -> Option<u16> {
    for (start, length) in SAVE_BLOCKS {
        match at.checked_sub(usize::from(length)) {
            Some(rest) => at = rest,
            None => return Some(start.0 + at as u16),
        }
    }
    None
}

impl Harness {
    /// The saved game the original save routine would write now.
    pub fn saved_game(&self) -> Vec<u8> {
        SAVE_BLOCKS
            .iter()
            .flat_map(|&(start, length)| start.0..start.0 + length)
            .map(|at| self.m.memory.read8(self.m.regs.ds, at))
            .collect()
    }

    /// Checks the saved game and the save index the game would read now
    /// against fixture `name`.
    ///
    /// # Panics
    ///
    /// When the fixture is missing, naming `cargo xtask fixtures`, or differs,
    /// naming the first data-segment address that does.
    pub fn assert_fixture(&self, name: &str) {
        let folder = fixtures().join(name);
        let read = |file: &str| {
            let path = folder.join(file);
            std::fs::read(&path).unwrap_or_else(|error| {
                panic!("{}: {error}; cargo xtask fixtures builds it", path.display())
            })
        };
        let (expected, saved) = (read(SAVED_GAME), self.saved_game());
        if let Some(at) =
            (0..expected.len().max(saved.len())).find(|&at| expected.get(at) != saved.get(at))
        {
            let address = address(at)
                .map_or_else(|| "beyond its blocks".to_owned(), |a| format!("DS:{a:04x}"));
            panic!("{name}: the saved game differs from the fixture first at {address}");
        }
        let files = &self.game.dos.files;
        let path = files.resolve(SAVE_INDEX.as_bytes(), false).expect("a plain file name");
        let index = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(index == read(SAVE_INDEX), "{name}: the save index differs from the fixture's");
    }
}

#[cfg(test)]
mod tests {
    use super::address;

    #[test]
    fn saved_game_bytes_name_their_data_segment_address() {
        assert_eq!(address(0), Some(0x5b4a));
        assert_eq!(address(0x36), Some(0x5e04), "the second block follows the first");
        assert_eq!(address(6065), Some(0x389c + 0x164f), "the last byte of the last block");
        assert_eq!(address(6066), None);
    }
}
