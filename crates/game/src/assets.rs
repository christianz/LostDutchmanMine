//! The game's compressed asset streams, decoded natively for the QoL overlay.
//!
//! A stream is a six-byte header (01 9D, packed size, declared size) and
//! MSB-first codes: below 256 a byte, 256 widens later codes by one bit, and
//! from 257 a dictionary entry. Each consecutive pair of decoded tokens
//! defines the next entry.

/// The first two header bytes.
const MAGIC: [u8; 2] = [1, 0x9d];
const HEADER: usize = 6;
const WIDEN: u32 = 256;
const FIRST_ENTRY: u32 = 257;
/// Codes never grow past 16 bits, and the output past 64 KiB.
const MAX_WIDTH: usize = 16;
const MAX_OUTPUT: usize = 65_535;

/// Why a stream cannot be decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AssetError {
    /// Not a well-formed asset stream.
    #[error("invalid LDM asset stream")]
    Invalid,
    /// The stream decodes to fewer bytes than its header declares.
    #[error("LDM asset shorter than its declared size")]
    Short,
}

/// Reads codes MSB-first.
struct Bits<'a> {
    data: &'a [u8],
    at: usize,
}

impl Bits<'_> {
    fn fits(&self, width: usize) -> bool {
        self.at + width <= self.data.len() * 8
    }

    fn read(&mut self, width: usize) -> u32 {
        let mut code = 0;
        for _ in 0..width {
            let bit = (self.data[self.at / 8] >> (7 - self.at % 8)) & 1;
            code = code << 1 | u32::from(bit);
            self.at += 1;
        }
        code
    }
}

/// Decodes a stream. The original decoder ignores the declared size once it is
/// reached: some shipped streams decode zero padding beyond it, so it is kept.
///
/// # Errors
///
/// [`AssetError`] for a malformed or short stream.
pub fn decode_asset(data: &[u8]) -> Result<Vec<u8>, AssetError> {
    if data.len() < HEADER || data[..2] != MAGIC {
        return Err(AssetError::Invalid);
    }
    let word = |at: usize| usize::from(u16::from_le_bytes([data[at], data[at + 1]]));
    let (packed, declared) = (word(2), word(4));
    if packed + HEADER != data.len() {
        return Err(AssetError::Invalid);
    }
    let mut out = Vec::with_capacity(declared);
    let mut entries: Vec<usize> = Vec::new();
    let mut bits = Bits { data, at: HEADER * 8 };
    let (mut width, mut tokens) = (9, 0usize);
    while bits.fits(width) {
        if tokens % 2 == 0 {
            entries.push(out.len());
        }
        let code = loop {
            if width > MAX_WIDTH {
                return Err(AssetError::Invalid);
            }
            if !bits.fits(width) {
                return if out.len() < declared { Err(AssetError::Invalid) } else { Ok(out) };
            }
            match bits.read(width) {
                WIDEN => width += 1,
                code => break code,
            }
        };
        if code < WIDEN {
            out.push(code as u8);
        } else {
            let entry = (code - FIRST_ENTRY) as usize;
            if entry + 1 >= entries.len() {
                return Err(AssetError::Invalid);
            }
            let (start, end) = (entries[entry], entries[entry + 1]);
            if end > out.len() || start >= end || end - start > MAX_OUTPUT - out.len() {
                return Err(AssetError::Invalid);
            }
            out.extend_from_within(start..end);
        }
        if out.len() > MAX_OUTPUT {
            return Err(AssetError::Invalid);
        }
        tokens += 1;
    }
    if out.len() < declared { Err(AssetError::Short) } else { Ok(out) }
}
