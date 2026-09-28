//! Every place the translated game differs from the supplied executable.
//!
//! The table is data: a site, the original bytes expected there, and a hook.
//! The translator applies it and checks every expected byte, so a patch can
//! never land on the wrong instruction. The `game` crate implements the hooks.

mod hook;
mod table;

pub use hook::{After, Hook, resume};
pub use table::PATCHES;

use machine::Address;

/// One change to one instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Patch {
    /// What the change is for.
    pub name: &'static str,
    /// The instruction it changes.
    pub site: Address,
    /// The instruction's original bytes.
    pub expect: &'static [u8],
    /// What happens there.
    pub action: Action,
}

/// What a patch does at its site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Run the hook, then the instruction, unless the hook jumps or yields.
    /// Several `Before` patches at one site run in table order.
    Before(Hook),
    /// Run the hook instead of the instruction. The hook may still ask for the
    /// original instruction with [`After::Original`].
    Replace(Hook),
}

impl Patch {
    /// The hook this patch runs.
    pub const fn hook(&self) -> Hook {
        match self.action {
            Action::Before(hook) | Action::Replace(hook) => hook,
        }
    }
}
