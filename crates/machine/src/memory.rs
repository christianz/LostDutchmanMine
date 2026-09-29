//! One megabyte of real-mode memory.

/// The size of the real-mode address space.
pub const SIZE: usize = 1 << 20;

/// Real-mode memory. Addresses wrap at one megabyte, and 16-bit accesses wrap
/// within their segment, exactly as on the 8086.
#[derive(Clone)]
pub struct Memory {
    bytes: Box<[u8; SIZE]>,
}

impl Memory {
    /// Zeroed memory, allocated directly on the heap.
    ///
    /// # Panics
    ///
    /// Never: the allocation has exactly one megabyte.
    pub fn new() -> Self {
        let bytes = vec![0; SIZE].into_boxed_slice().try_into().expect("exactly one megabyte");
        Memory { bytes }
    }

    /// The linear address of `segment:offset`.
    pub const fn linear(segment: u16, offset: u16) -> usize {
        ((segment as usize) << 4).wrapping_add(offset as usize) & (SIZE - 1)
    }

    /// Reads a byte.
    pub fn read8(&self, segment: u16, offset: u16) -> u8 {
        self.bytes[Self::linear(segment, offset)]
    }

    /// Reads a little-endian word; its second byte wraps within the segment.
    pub fn read16(&self, segment: u16, offset: u16) -> u16 {
        u16::from_le_bytes([
            self.read8(segment, offset),
            self.read8(segment, offset.wrapping_add(1)),
        ])
    }

    /// Writes a byte.
    pub fn write8(&mut self, segment: u16, offset: u16, value: u8) {
        self.bytes[Self::linear(segment, offset)] = value;
    }

    /// Writes a little-endian word; its second byte wraps within the segment.
    pub fn write16(&mut self, segment: u16, offset: u16, value: u16) {
        let [low, high] = value.to_le_bytes();
        self.write8(segment, offset, low);
        self.write8(segment, offset.wrapping_add(1), high);
    }

    /// All of memory, by linear address.
    pub fn as_bytes(&self) -> &[u8; SIZE] {
        &self.bytes
    }

    /// All of memory, by linear address, for loading images and bulk copies.
    pub fn as_bytes_mut(&mut self) -> &mut [u8; SIZE] {
        &mut self.bytes
    }
}

impl Default for Memory {
    fn default() -> Self {
        Memory::new()
    }
}

impl std::fmt::Debug for Memory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Memory(1 MiB)")
    }
}
