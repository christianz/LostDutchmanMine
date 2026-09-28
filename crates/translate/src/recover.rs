//! Control-flow recovery: which bytes of the image are code.
//!
//! A conservative recursive descent, like the original `tools/analyze.py`: start
//! from the entry point, relocated far references and evidence-based entry
//! points, and follow every direct jump and call. Indirect transfers are
//! recorded, never guessed.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use iced_x86::{Decoder, DecoderOptions, Mnemonic, OpKind};
use machine::Address;
use serde::Deserialize;

use crate::image::LoadImage;

/// The recovered code.
#[derive(Clone, Debug, Default)]
pub struct Recovered {
    /// Every recovered instruction and its length.
    pub instructions: BTreeMap<Address, u16>,
    /// Every address recovery started from: seeds and direct transfer targets.
    pub targets: BTreeSet<Address>,
    /// Indirect jumps and calls, whose targets need evidence to recover.
    pub indirect: Vec<Address>,
    /// Addresses that were reached but do not decode.
    pub invalid: Vec<Address>,
}

/// Recovers the code reachable from the entry point, far references and `seeds`.
pub fn recover(image: &LoadImage, seeds: &[Address]) -> Recovered {
    let bytes = &image.bytes;
    let mut starts: BTreeSet<Address> = [image.entry, Address::new(0, 0)].into();
    starts.extend(seeds);
    // A relocated word right after a far CALL or JMP opcode is the segment of
    // its target; the offset is the word before it.
    for &at in &image.relocations {
        let at = at as usize;
        if at >= 3 && matches!(bytes[at - 3], 0x9a | 0xea) {
            let word = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
            let (offset, segment) = (word(at - 2), word(at));
            if u32::from(segment) * 16 + u32::from(offset) < 0x2_0000 {
                starts.insert(Address::new(segment, offset));
            }
        }
    }

    let mut recovered = Recovered { targets: starts.clone(), ..Recovered::default() };
    let mut queue: VecDeque<Address> = starts.into_iter().collect();
    while let Some(start) = queue.pop_front() {
        let mut at = start;
        while !recovered.instructions.contains_key(&at) {
            let linear = usize::from(at.segment) * 16 + usize::from(at.offset);
            let window = bytes.get(linear..bytes.len().min(linear + 15)).unwrap_or_default();
            let decoded =
                Decoder::with_ip(16, window, u64::from(at.offset), DecoderOptions::NONE).decode();
            if window.is_empty() || decoded.is_invalid() {
                recovered.invalid.push(at);
                break;
            }
            recovered.instructions.insert(at, decoded.len() as u16);
            let mnemonic = decoded.mnemonic();
            let transfers = matches!(mnemonic, Mnemonic::Jmp | Mnemonic::Call)
                || decoded.op_count() > 0 && decoded.op0_kind() == OpKind::NearBranch16;
            if transfers {
                let target = match decoded.op0_kind() {
                    OpKind::NearBranch16 => Some(Address::new(at.segment, decoded.near_branch16())),
                    OpKind::FarBranch16 => {
                        Some(Address::new(decoded.far_branch_selector(), decoded.far_branch16()))
                    }
                    _ => None,
                };
                match target {
                    Some(target) => {
                        recovered.targets.insert(target);
                        queue.push_back(target);
                    }
                    None => recovered.indirect.push(at),
                }
                if mnemonic == Mnemonic::Jmp {
                    break;
                }
            }
            if matches!(mnemonic, Mnemonic::Ret | Mnemonic::Retf | Mnemonic::Iret | Mnemonic::Hlt) {
                break;
            }
            at = Address::new(at.segment, at.offset.wrapping_add(decoded.len() as u16));
        }
    }
    recovered
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryPoints {
    group: Vec<Group>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Group {
    #[allow(dead_code, reason = "Required documentation: every group states its evidence.")]
    why: String,
    at: Vec<String>,
}

/// Why an entry-point list cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum EntryPointError {
    /// Not valid TOML, or a group without `why` or `at`.
    #[error("invalid entry points: {0}")]
    Toml(#[from] toml::de::Error),
    /// An address that is not `ssss:oooo` in hexadecimal.
    #[error("invalid address `{0}`: expected ssss:oooo in hexadecimal")]
    Address(String),
}

/// Parses `entry_points.toml`: groups of addresses, each with its evidence.
///
/// # Errors
///
/// [`EntryPointError`] for malformed TOML, missing reasons or addresses.
pub fn parse_entry_points(text: &str) -> Result<Vec<Address>, EntryPointError> {
    let points: EntryPoints = toml::from_str(text)?;
    let mut addresses = Vec::new();
    for text in points.group.iter().flat_map(|group| &group.at) {
        let parsed = text.split_once(':').and_then(|(s, o)| {
            Some(Address::new(u16::from_str_radix(s, 16).ok()?, u16::from_str_radix(o, 16).ok()?))
        });
        addresses.push(parsed.ok_or_else(|| EntryPointError::Address(text.clone()))?);
    }
    Ok(addresses)
}
