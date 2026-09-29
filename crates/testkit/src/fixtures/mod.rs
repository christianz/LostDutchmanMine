//! Scenario fixtures: isolated saves built from the player's own game.
//!
//! Fixtures hold game data, so they are built into `.local/fixtures` and never
//! committed. Each is built twice and accepted only when both builds are byte
//! for byte the same, so scenarios start from exactly the same state anywhere.

mod save;

use std::path::{Path, PathBuf};

use game::symbols::{
    BUILDING, BULLETS, ENCOUNTER_KIND, GUNS, ITEM_PAN, PANS, POSITION_X, POSITION_Y, SCENE_CAVE,
    SCENE_ENCOUNTER, SCENE_MAP, SCENE_RIVER, SCENE_TOWN, SCENE_VARIANT, inventory,
};

pub use save::SaveGame;

use crate::harness::Harness;
use crate::moments::cave::{Cave, MINING_POSITION, PICK};
use crate::moments::{assay, river};

/// Why a fixture could not be built.
#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    /// A file could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// The file.
        path: PathBuf,
        /// The failure.
        source: std::io::Error,
    },
    /// The player's files cannot make this fixture.
    #[error("{0}")]
    Invalid(String),
}

impl FixtureError {
    pub(crate) fn io(path: &Path, source: std::io::Error) -> Self {
        FixtureError::Io { path: path.to_path_buf(), source }
    }
}

/// The saved game every fixture starts from: the player's first slot.
const SOURCE: &str = "LDMSAVE1.SAV";
/// The saves index the game reads beside the slots.
const INDEX: &str = "LDMSAVE.LDM";

/// Every fixture, each into its own folder under `out`, from the player's
/// game in `data`; the ones taken from a running game use the harness's.
///
/// # Errors
///
/// When the player's saves cannot make a fixture, or files fail.
pub fn build_all(data: &Path, out: &Path) -> Result<(), FixtureError> {
    combat(data, &out.join("combat"))?;
    river(data, &out.join("river"))?;
    export(&assay::office(true), &out.join("assay"))?;
    let mut cave = Cave::new();
    export(&cave.h, &out.join("map"))?;
    cave.enter();
    cave.h.set(inventory(2, 0), PICK);
    cave.h.set(POSITION_X, MINING_POSITION.0);
    cave.h.set(POSITION_Y, MINING_POSITION.1);
    export(&cave.h, &out.join("mining"))?;
    let mut h = river::town(true);
    river::river_state(&mut h);
    // The counter says a pan, but the pack holds none, as the stale counter
    // a rejected full-pack purchase leaves.
    h.set(PANS, 1);
    export(&h, &out.join("pan-missing"))?;
    h.set(inventory(1, 0), ITEM_PAN);
    export(&h, &out.join("pan-owned"))
}

/// Saves the running game as slot 1 in a new folder `out`, beside the save
/// index the game would read.
fn export(h: &Harness, out: &Path) -> Result<(), FixtureError> {
    std::fs::create_dir_all(out).map_err(|source| FixtureError::io(out, source))?;
    let slot = out.join(SOURCE);
    std::fs::write(&slot, h.saved_game()).map_err(|source| FixtureError::io(&slot, source))?;
    let index = h
        .game
        .dos
        .files
        .resolve(INDEX.as_bytes(), false)
        .map_err(|error| FixtureError::Invalid(error.to_string()))?;
    std::fs::copy(&index, out.join(INDEX)).map_err(|source| FixtureError::io(&index, source))?;
    Ok(())
}

/// Writes `save` as slot 1 in a new folder `out`, beside the player's index.
fn write_fixture(data: &Path, save: &SaveGame, out: &Path) -> Result<(), FixtureError> {
    std::fs::create_dir_all(out).map_err(|source| FixtureError::io(out, source))?;
    save.write(&out.join(SOURCE))?;
    let (from, to) = (data.join(INDEX), out.join(INDEX));
    std::fs::copy(&from, &to).map_err(|source| FixtureError::io(&from, source))?;
    Ok(())
}

/// Leaves every scene, then shows the one `scene` names.
fn only_scene(save: &mut SaveGame, scene: game::symbols::Global) {
    for flag in [SCENE_TOWN, SCENE_MAP, SCENE_RIVER, SCENE_CAVE, SCENE_ENCOUNTER] {
        save.set(flag, u16::from(flag == scene));
    }
}

/// An armed encounter: slot 1 in a bandit fight, sight centred. The bandit and
/// Native American fights share routine 040a:0002; the encounter's type is not
/// saved, so loading this starts the default bandit fight.
///
/// # Errors
///
/// When slot 1 has no gun and ammunition, or files fail.
pub fn combat(data: &Path, out: &Path) -> Result<(), FixtureError> {
    let mut save = SaveGame::read(&data.join(SOURCE))?;
    if save.word(GUNS) == 0 || save.word(BULLETS) == 0 {
        return Err(FixtureError::Invalid("slot 1 must own a gun and ammunition".to_owned()));
    }
    save.set(POSITION_X, 160);
    save.set(POSITION_Y, 64);
    save.set(BUILDING, 0);
    save.set(ENCOUNTER_KIND, 0);
    only_scene(&mut save, SCENE_ENCOUNTER);
    write_fixture(data, &save, out)
}

/// A river: slot 1 standing at the river, which the original routines
/// 0e5a:0140 and 0e5a:0380 save and load like any scene.
///
/// # Errors
///
/// When slot 1 owns no pan, or files fail.
pub fn river(data: &Path, out: &Path) -> Result<(), FixtureError> {
    let mut save = SaveGame::read(&data.join(SOURCE))?;
    if save.word(PANS) == 0 {
        return Err(FixtureError::Invalid("slot 1 must own a pan".to_owned()));
    }
    save.set(POSITION_X, 40);
    save.set(POSITION_Y, 55);
    save.set(BUILDING, 0);
    save.set(SCENE_VARIANT, 0);
    only_scene(&mut save, SCENE_RIVER);
    write_fixture(data, &save, out)
}
