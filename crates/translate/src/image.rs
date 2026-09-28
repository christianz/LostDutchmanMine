//! The game's executable: identification and EXEPACK unpacking.
//!
//! An independent implementation of the documented EXEPACK format
//! (<https://github.com/w4kfu/unEXEPACK#exepack-header>). No DOS code runs.

use machine::Address;
use sha2::{Digest, Sha256};

/// SHA-256 of the one supported `LDM.EXE`.
pub const SUPPORTED_SHA256: &str =
    "de0726a1cb0a475cd05f19ffb56e6c84f014fdb34f986d374d5e09797b925e07";

/// The unpacked program image and what the loader needs to start it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadImage {
    /// The program image, unrelocated.
    pub bytes: Vec<u8>,
    /// Image offsets of the words the loader relocates.
    pub relocations: Vec<u32>,
    /// The entry point, image-relative.
    pub entry: Address,
    /// The initial stack segment, image-relative.
    pub stack_segment: u16,
    /// The initial stack pointer.
    pub stack_pointer: u16,
}

/// Why an executable cannot be used.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ImageError {
    /// Not a DOS MZ executable.
    #[error("not an MZ executable")]
    NotMz,
    /// A different executable than the supported release.
    #[error("not the supported LDM.EXE: SHA-256 {found}, expected {SUPPORTED_SHA256}")]
    UnsupportedRelease {
        /// The file's SHA-256.
        found: String,
    },
    /// The EXEPACK data is malformed.
    #[error("invalid EXEPACK data: {0}")]
    Invalid(&'static str),
}

/// Checks that `exe` is the supported release, then unpacks it.
///
/// # Errors
///
/// [`ImageError::UnsupportedRelease`] for any other file, or an unpacking error.
pub fn load(exe: &[u8]) -> Result<LoadImage, ImageError> {
    let found = sha256_hex(exe);
    if found != SUPPORTED_SHA256 {
        return Err(ImageError::UnsupportedRelease { found });
    }
    unpack(exe)
}

/// The lowercase hexadecimal SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// Lowercase hexadecimal, two digits per byte.
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

fn word(data: &[u8], at: usize) -> Result<u16, ImageError> {
    data.get(at..at + 2)
        .map(|w| u16::from_le_bytes([w[0], w[1]]))
        .ok_or(ImageError::Invalid("truncated"))
}

/// Unpacks an EXEPACK executable of the "RB", skip-1 variant.
///
/// # Errors
///
/// [`ImageError`] describing the first malformed structure.
pub fn unpack(data: &[u8]) -> Result<LoadImage, ImageError> {
    use ImageError::Invalid;
    if data.len() < 28 || &data[..2] != b"MZ" {
        return Err(ImageError::NotMz);
    }
    let start = usize::from(word(data, 8)?) * 16;
    let stub = start + usize::from(word(data, 22)?) * 16;
    if stub + 18 > data.len() {
        return Err(Invalid("truncated EXEPACK header"));
    }
    let field = |n: usize| word(data, stub + 2 * n);
    let (ip, cs, size, sp, ss, paragraphs, skip, signature) =
        (field(0)?, field(1)?, field(3)?, field(4)?, field(5)?, field(6)?, field(7)?, field(8)?);
    if signature != 0x4252 || skip != 1 {
        return Err(Invalid("unsupported EXEPACK variant (expected RB, skip=1)"));
    }
    let end = stub + usize::from(size);
    if end > data.len() {
        return Err(Invalid("truncated EXEPACK data"));
    }

    // Commands run backwards from the stub, filling the output from its end.
    let mut output = vec![0; usize::from(paragraphs) * 16];
    let (mut src, mut dst) = (stub, output.len());
    while src > start && data[src - 1] == 0xff {
        src -= 1;
    }
    loop {
        if src < start + 3 {
            return Err(Invalid("truncated compression command"));
        }
        let op = data[src - 1];
        let count = usize::from(word(data, src - 3)?);
        src -= 3;
        if count > dst {
            return Err(Invalid("compression command exceeds the declared output"));
        }
        match op & 0xfe {
            0xb0 => {
                if src <= start {
                    return Err(Invalid("missing fill byte"));
                }
                src -= 1;
                output[dst - count..dst].fill(data[src]);
            }
            0xb2 => {
                if src < start + count {
                    return Err(Invalid("literal run exceeds the compressed input"));
                }
                output[dst - count..dst].copy_from_slice(&data[src - count..src]);
                src -= count;
            }
            _ => return Err(Invalid("unknown compression command")),
        }
        dst -= count;
        if op & 1 != 0 {
            break;
        }
    }
    if src - start != dst {
        return Err(Invalid("unpacked size does not match the stored prefix"));
    }
    output[..dst].copy_from_slice(&data[start..src]);

    // Sixteen groups of relocations follow the stub's error message.
    let marker = b"Packed file is corrupt";
    let found = data[stub..end].windows(marker.len()).position(|w| w == marker);
    let mut at = stub + found.ok_or(Invalid("missing relocation marker"))? + marker.len();
    let mut relocations = Vec::new();
    for group in 0..16_u32 {
        let count = word(data, at).map_err(|_| Invalid("truncated relocation table"))?;
        at += 2;
        if at + 2 * usize::from(count) > end {
            return Err(Invalid("truncated relocation group"));
        }
        for _ in 0..count {
            let offset = u32::from(word(data, at)?) + group * 0x1_0000;
            at += 2;
            if offset as usize + 2 > output.len() {
                return Err(Invalid("relocation outside the load image"));
            }
            relocations.push(offset);
        }
    }
    if at != end {
        return Err(Invalid("unexpected data after the relocation table"));
    }
    Ok(LoadImage {
        bytes: output,
        relocations,
        entry: Address::new(cs, ip),
        stack_segment: ss,
        stack_pointer: sp,
    })
}
