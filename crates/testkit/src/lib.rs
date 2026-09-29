//! Test support: the IR reference interpreter, hardware test vectors, scenario
//! runs against the C++ oracle's golden traces, the checks ported from its
//! scenario verifiers, and the step-by-step harness its native tests ran on.

pub mod checks;
pub mod harness;
pub mod interp;
pub mod scenario;
pub mod script;
pub mod vectors;
