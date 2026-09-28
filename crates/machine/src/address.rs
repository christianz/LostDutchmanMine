//! Addresses within the game's program image.

use crate::LOAD_SEGMENT;

/// A `segment:offset` address relative to the start of the program image, as
/// the disassembly, patch table and diagnostics write it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Address {
    /// Paragraphs from the start of the program image.
    pub segment: u16,
    /// Offset within the segment.
    pub offset: u16,
}

impl Address {
    /// An image-relative address.
    pub const fn new(segment: u16, offset: u16) -> Self {
        Address { segment, offset }
    }

    /// The image-relative address of a running `cs:ip`.
    pub const fn from_runtime(cs: u16, ip: u16) -> Self {
        Address { segment: cs.wrapping_sub(LOAD_SEGMENT), offset: ip }
    }

    /// The runtime code segment of this address.
    pub const fn runtime_segment(self) -> u16 {
        self.segment.wrapping_add(LOAD_SEGMENT)
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04x}:{:04x}", self.segment, self.offset)
    }
}
