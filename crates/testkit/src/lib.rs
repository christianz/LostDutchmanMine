//! Test support: the IR reference interpreter, hardware test vectors, scenario
//! runs against the C++ oracle's golden traces, and the checks ported from its
//! scenario verifiers.

pub mod checks;
pub mod interp;
pub mod scenario;
pub mod script;
pub mod vectors;
