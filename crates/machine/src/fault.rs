//! CPU faults the translated game can raise.

/// A fault raised by an instruction. The original game never relies on the
/// processor's divide-error interrupt, so these stop execution with a diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Fault {
    /// DIV or IDIV by zero.
    #[error("division by zero")]
    DivideByZero,
    /// DIV or IDIV whose quotient does not fit the destination.
    #[error("division overflow")]
    DivideOverflow,
}
