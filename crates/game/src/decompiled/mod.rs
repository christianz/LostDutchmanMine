//! Original routines as readable Rust, each standing in for its translation
//! from entry to return. Each does exactly what its original does to memory,
//! registers and the flags its callers read; `crate::routines` checks that in
//! lockstep against the translation.

pub(crate) mod assets;

#[cfg(test)]
mod tests;

use machine::Machine;
use patches::Routine;

/// Runs `routine`, including its return; false when it declines this call,
/// changing nothing, so the translation runs instead.
pub(crate) fn run(routine: Routine, m: &mut Machine) -> bool {
    match routine {
        Routine::DecodeAsset => assets::decode(m),
    }
}

/// Whether `routine` returns far, popping CS as well as IP.
pub(crate) const fn returns_far(routine: Routine) -> bool {
    match routine {
        Routine::DecodeAsset => false,
    }
}
