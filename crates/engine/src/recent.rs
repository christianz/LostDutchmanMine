//! Where the last translated steps began, for reports of how a run stopped.

use machine::{Address, Machine};

/// How many steps are remembered.
const REMEMBERED: usize = 64;

/// A ring of the most recent step addresses.
#[derive(Clone, Debug)]
pub(crate) struct Recent {
    addresses: [Address; REMEMBERED],
    noted: usize,
}

impl Default for Recent {
    fn default() -> Self {
        Recent { addresses: [Address::new(0, 0); REMEMBERED], noted: 0 }
    }
}

impl Recent {
    /// A step begins at CS:IP.
    pub(crate) fn note(&mut self, m: &Machine) {
        self.addresses[self.noted % REMEMBERED] = Address::from_runtime(m.regs.cs, m.regs.ip);
        self.noted += 1;
    }

    /// The remembered addresses, oldest first.
    pub(crate) fn oldest_first(&self) -> Vec<Address> {
        let kept = self.noted.min(REMEMBERED);
        (self.noted - kept..self.noted).map(|i| self.addresses[i % REMEMBERED]).collect()
    }
}
