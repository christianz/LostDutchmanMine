//! Routines the port can run as readable Rust instead of their translation.

/// A whole original routine, from its entry to its return, that readable Rust
/// can stand in for. Its contract, what callers may observe, is computed by
/// the translator and checked by running both in lockstep.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Routine {
    /// 1265:1250, near: decodes the packed asset stream at DS:SI to ES:DI.
    /// AX is the end of the output and CF is set for a foreign stream.
    DecodeAsset,
}
