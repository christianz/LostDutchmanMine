//! One scenario's run as the checks read it: its captures, and the settings
//! file and saves it left in its folder.

use std::collections::BTreeMap;
use std::path::PathBuf;

use game::Report;

use crate::scenario::{Capture, Outcome};

/// A scenario's outcome and the folder it ran in.
#[derive(Debug)]
pub struct Run {
    /// What the run produced.
    pub outcome: Outcome,
    /// Its folder, holding `Saves/` and perhaps `display.ini`.
    pub folder: PathBuf,
}

impl Run {
    /// Capture `id`.
    ///
    /// # Errors
    ///
    /// A message naming the capture when the script made none with that number.
    pub fn capture(&self, id: u32) -> Result<&Capture, String> {
        self.outcome
            .captures
            .iter()
            .find(|capture| capture.id == id)
            .ok_or_else(|| format!("no capture {id}"))
    }

    /// The state at capture `id`.
    ///
    /// # Errors
    ///
    /// As [`Run::capture`].
    pub fn report(&self, id: u32) -> Result<&Report, String> {
        self.capture(id).map(|capture| &capture.report)
    }

    /// The states at captures `ids`, in order.
    ///
    /// # Errors
    ///
    /// As [`Run::capture`], for the first capture missing.
    pub fn reports<const N: usize>(&self, ids: [u32; N]) -> Result<[&Report; N], String> {
        reports(ids, |id| self.report(id))
    }

    /// The settings file's `key=value` lines, skipping `#` comments, as the
    /// verifiers read it: a later line for a key replaces an earlier one.
    ///
    /// # Errors
    ///
    /// A message naming the file when it cannot be read.
    pub fn settings(&self) -> Result<BTreeMap<String, String>, String> {
        let path = self.folder.join("display.ini");
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(text
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| line.split_once('='))
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect())
    }

    /// The saved-game file `name`.
    ///
    /// # Errors
    ///
    /// A message naming the file when it cannot be read.
    pub fn save(&self, name: &str) -> Result<Vec<u8>, String> {
        let path = self.folder.join("Saves").join(name);
        std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// The states at captures `ids`, in order, each found by `report`.
///
/// # Errors
///
/// The first error `report` returns.
pub fn reports<'a, const N: usize>(
    ids: [u32; N],
    report: impl Fn(u32) -> Result<&'a Report, String>,
) -> Result<[&'a Report; N], String> {
    let reports: Vec<&Report> = ids.into_iter().map(report).collect::<Result<_, _>>()?;
    reports.try_into().map_err(|_| format!("not {N} captures"))
}

/// `N` consecutive capture numbers from `first`.
pub fn ids<const N: usize>(first: u32) -> [u32; N] {
    let mut ids = [first; N];
    for (id, next) in ids.iter_mut().zip(first..) {
        *id = next;
    }
    ids
}
