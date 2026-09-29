//! Ahead-of-time translation of Lost Dutchman Mine's 8086 code into Rust.
//!
//! Instructions are decoded once, at build time, lowered into a small IR whose
//! meaning is defined by the `machine` crate, and written out as Rust.

pub mod emit;
pub mod image;
pub mod ir;
pub mod liveness;
mod lower;
pub mod recover;
pub mod translation;

pub use lower::{Instruction, LowerError, Relocations, lower};
