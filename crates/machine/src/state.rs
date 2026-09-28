//! Savestates: one versioned little-endian encoding that each layer writes its
//! own part of, in order, behind a four-byte section tag.
//!
//! A state holds game memory, so it stays on the player's machine; it is
//! never committed or shared. [`VERSION`] changes whenever any layer's
//! encoding does, and older states are refused rather than misread.

use crate::{Machine, Registers};

/// The encoding's version.
pub const VERSION: u32 = 1;
const MAGIC: &[u8; 8] = b"LDMSTATE";

/// Why a state cannot be restored.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StateError {
    /// Not a savestate at all.
    #[error("not a savestate")]
    NotAState,
    /// A savestate of another version.
    #[error("a version {0} savestate; this build reads version {VERSION}")]
    Version(u32),
    /// The state ends early.
    #[error("the savestate is cut short")]
    Truncated,
    /// The state goes on after everything was read.
    #[error("the savestate has data after its end")]
    Trailing,
    /// A section or value that cannot be.
    #[error("the savestate is damaged: {0}")]
    Invalid(String),
}

/// A layer's state, saved and restored in place.
pub trait Persist {
    /// Writes this layer's state.
    fn save(&self, w: &mut Writer);

    /// Reads the state [`Persist::save`] wrote, replacing this one's.
    ///
    /// # Errors
    ///
    /// [`StateError`] when the state is damaged.
    fn restore(&mut self, r: &mut Reader) -> Result<(), StateError>;
}

/// Encodes a state.
#[derive(Clone, Debug)]
pub struct Writer {
    bytes: Vec<u8>,
}

impl Default for Writer {
    fn default() -> Self {
        Writer::new()
    }
}

impl Writer {
    /// A state with its header written.
    pub fn new() -> Self {
        let mut writer = Writer { bytes: MAGIC.to_vec() };
        writer.u32(VERSION);
        writer
    }

    /// The encoded state.
    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }

    /// Starts a layer's section.
    pub fn section(&mut self, tag: &[u8; 4]) {
        self.bytes.extend_from_slice(tag);
    }

    /// A byte.
    pub fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    /// A flag.
    pub fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
    }

    /// A word.
    pub fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// A double word.
    pub fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// A quad word.
    pub fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// A signed double word.
    pub fn i32(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// A signed quad word.
    pub fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// Bytes whose length the reader knows.
    pub fn fixed(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    /// Bytes of any length.
    ///
    /// # Panics
    ///
    /// For 4 GiB or more, which no part of the machine holds.
    pub fn bytes(&mut self, bytes: &[u8]) {
        self.u32(u32::try_from(bytes.len()).expect("a savestate field under 4 GiB"));
        self.fixed(bytes);
    }
}

/// Decodes a state.
#[derive(Clone, Debug)]
pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    /// Reads the header.
    ///
    /// # Errors
    ///
    /// [`StateError::NotAState`] or [`StateError::Version`].
    pub fn new(bytes: &'a [u8]) -> Result<Self, StateError> {
        if !bytes.starts_with(MAGIC) {
            return Err(StateError::NotAState);
        }
        let mut reader = Reader { bytes, at: MAGIC.len() };
        match reader.u32()? {
            VERSION => Ok(reader),
            other => Err(StateError::Version(other)),
        }
    }

    /// Checks nothing is left.
    ///
    /// # Errors
    ///
    /// [`StateError::Trailing`] when something is.
    pub fn finish(self) -> Result<(), StateError> {
        if self.at == self.bytes.len() { Ok(()) } else { Err(StateError::Trailing) }
    }

    /// Checks a layer's section starts here.
    ///
    /// # Errors
    ///
    /// [`StateError::Invalid`] for another section.
    pub fn section(&mut self, tag: &[u8; 4]) -> Result<(), StateError> {
        if self.fixed(4)? == tag {
            Ok(())
        } else {
            Err(StateError::Invalid(format!(
                "expected the {} section",
                String::from_utf8_lossy(tag)
            )))
        }
    }

    /// Bytes whose length is known.
    ///
    /// # Errors
    ///
    /// [`StateError::Truncated`].
    pub fn fixed(&mut self, length: usize) -> Result<&'a [u8], StateError> {
        let end = self.at.checked_add(length).filter(|&end| end <= self.bytes.len());
        let end = end.ok_or(StateError::Truncated)?;
        let bytes = &self.bytes[self.at..end];
        self.at = end;
        Ok(bytes)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], StateError> {
        Ok(self.fixed(N)?.try_into().expect("exactly N bytes"))
    }

    /// A byte.
    ///
    /// # Errors
    ///
    /// [`StateError::Truncated`].
    pub fn u8(&mut self) -> Result<u8, StateError> {
        Ok(self.array::<1>()?[0])
    }

    /// A flag.
    ///
    /// # Errors
    ///
    /// [`StateError::Truncated`], or [`StateError::Invalid`] for other than 0 and 1.
    pub fn bool(&mut self) -> Result<bool, StateError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(StateError::Invalid(format!("{other} is not a flag"))),
        }
    }

    /// A word.
    ///
    /// # Errors
    ///
    /// [`StateError::Truncated`].
    pub fn u16(&mut self) -> Result<u16, StateError> {
        self.array().map(u16::from_le_bytes)
    }

    /// A double word.
    ///
    /// # Errors
    ///
    /// [`StateError::Truncated`].
    pub fn u32(&mut self) -> Result<u32, StateError> {
        self.array().map(u32::from_le_bytes)
    }

    /// A quad word.
    ///
    /// # Errors
    ///
    /// [`StateError::Truncated`].
    pub fn u64(&mut self) -> Result<u64, StateError> {
        self.array().map(u64::from_le_bytes)
    }

    /// A signed double word.
    ///
    /// # Errors
    ///
    /// [`StateError::Truncated`].
    pub fn i32(&mut self) -> Result<i32, StateError> {
        self.array().map(i32::from_le_bytes)
    }

    /// A signed quad word.
    ///
    /// # Errors
    ///
    /// [`StateError::Truncated`].
    pub fn i64(&mut self) -> Result<i64, StateError> {
        self.array().map(i64::from_le_bytes)
    }

    /// Bytes of any length.
    ///
    /// # Errors
    ///
    /// [`StateError::Truncated`].
    pub fn bytes(&mut self) -> Result<&'a [u8], StateError> {
        let length = self.u32()? as usize;
        self.fixed(length)
    }
}

impl Persist for Machine {
    fn save(&self, w: &mut Writer) {
        w.section(b"MACH");
        for register in self.regs.in_trace_order() {
            w.u16(register);
        }
        w.fixed(self.memory.as_bytes());
        self.ports.save(w);
        self.pit.save(w);
        w.bytes(&self.opl.save());
        w.u64(self.opl.data_writes());
        for colour in self.vga.palette {
            w.u32(colour);
        }
        w.fixed(&self.vga.attributes);
        w.u64(self.steps);
    }

    fn restore(&mut self, r: &mut Reader) -> Result<(), StateError> {
        r.section(b"MACH")?;
        let mut registers = [0; 14];
        for register in &mut registers {
            *register = r.u16()?;
        }
        self.regs = Registers::from_trace_order(registers);
        let memory = r.fixed(crate::MEMORY_SIZE)?;
        self.memory.as_bytes_mut().copy_from_slice(memory);
        self.ports.restore(r)?;
        self.pit.restore(r)?;
        let chip = r.bytes()?;
        let data_writes = r.u64()?;
        self.opl.restore_with_count(chip, data_writes);
        for colour in &mut self.vga.palette {
            *colour = r.u32()?;
        }
        self.vga.attributes.copy_from_slice(r.fixed(16)?);
        self.steps = r.u64()?;
        Ok(())
    }
}
