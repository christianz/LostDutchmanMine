//! Test support: the IR reference interpreter, hardware test vectors, scenario
//! runs against the C++ oracle's golden traces, the checks ported from its
//! scenario verifiers, the step-by-step harness its native tests ran on, the
//! moments those tests start from, and the scenario fixtures built from them.

pub mod checks;
pub mod fixtures;
pub mod harness;
pub mod interp;
pub mod moments;
pub mod scenario;
pub mod script;
pub mod vectors;
