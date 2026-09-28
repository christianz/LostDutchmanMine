//! A complete translation: every recovered instruction lowered, every patch
//! checked and placed, and the offsets where translated blocks begin.

use std::collections::{BTreeMap, BTreeSet};

use machine::Address;
use patches::{Action, Patch};

use crate::image::{LoadImage, hex};
use crate::ir::Ir;
use crate::recover::Recovered;
use crate::{Instruction, LowerError, Relocations, lower};

/// One code segment of the translation.
#[derive(Clone, Debug, Default)]
pub struct Segment {
    /// The image-relative segment.
    pub number: u16,
    /// Its instructions by offset.
    pub instructions: BTreeMap<u16, Instruction>,
    /// Patches by site offset, in application order.
    pub patches: BTreeMap<u16, Vec<Patch>>,
    /// Offsets where execution can begin or resume: each starts a block.
    pub block_starts: BTreeSet<u16>,
}

/// The whole program, ready to be written out.
#[derive(Clone, Debug, Default)]
pub struct Translation {
    /// Segments by image-relative number.
    pub segments: BTreeMap<u16, Segment>,
}

/// Why a translation cannot be built.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TranslationError {
    /// A recovered instruction the translator cannot lower.
    #[error(transparent)]
    Lower(#[from] LowerError),
    /// A patch whose site is not a recovered instruction.
    #[error("patch `{name}` at {site}: not the start of a recovered instruction")]
    NotAnInstruction {
        /// The patch.
        name: &'static str,
        /// Its site.
        site: Address,
    },
    /// A patch whose expected bytes differ from the executable's.
    #[error("patch `{name}` at {site}: expected {expected}, found {found}")]
    UnexpectedBytes {
        /// The patch.
        name: &'static str,
        /// Its site.
        site: Address,
        /// The bytes it expects.
        expected: String,
        /// The bytes there.
        found: String,
    },
    /// A patch that may continue somewhere no instruction was recovered.
    #[error("patch `{name}` at {site}: target {target:04x} is not recovered code")]
    TargetNotCode {
        /// The patch.
        name: &'static str,
        /// Its site.
        site: Address,
        /// The offset.
        target: u16,
    },
}

impl Translation {
    /// Lowers every recovered instruction and places the patches.
    ///
    /// # Errors
    ///
    /// [`TranslationError`] for an instruction that does not lower or a patch
    /// that does not fit the executable.
    pub fn build(
        image: &LoadImage,
        recovered: &Recovered,
        patches: &[Patch],
    ) -> Result<Self, TranslationError> {
        let relocations = Relocations::from_offsets(image.relocations.iter().copied());
        let bytes_at = |at: Address, length: usize| {
            let linear = usize::from(at.segment) * 16 + usize::from(at.offset);
            &image.bytes[linear..image.bytes.len().min(linear + length)]
        };
        let mut translation = Translation::default();
        for &at in recovered.instructions.keys() {
            let instruction = lower(at, bytes_at(at, 15), &relocations)?;
            let segment = translation.segment(at.segment);
            segment.instructions.insert(at.offset, instruction);
        }
        for &target in &recovered.targets {
            if let Some(segment) = translation.segments.get_mut(&target.segment) {
                segment.block_starts.insert(target.offset);
            }
        }
        for patch in patches {
            translation.place(patch, bytes_at(patch.site, patch.expect.len()))?;
        }
        for segment in translation.segments.values_mut() {
            segment.add_resume_points();
        }
        Ok(translation)
    }

    fn segment(&mut self, number: u16) -> &mut Segment {
        self.segments.entry(number).or_insert_with(|| Segment { number, ..Segment::default() })
    }

    fn place(&mut self, patch: &Patch, found: &[u8]) -> Result<(), TranslationError> {
        let (name, site) = (patch.name, patch.site);
        let segment = self.segments.get_mut(&site.segment);
        let Some(instruction) = segment.as_ref().and_then(|s| s.instructions.get(&site.offset))
        else {
            return Err(TranslationError::NotAnInstruction { name, site });
        };
        if found != patch.expect || usize::from(instruction.length) != patch.expect.len() {
            let (expected, found) = (hex(patch.expect), hex(found));
            return Err(TranslationError::UnexpectedBytes { name, site, expected, found });
        }
        let next = instruction.next();
        let segment = segment.expect("checked above");
        for target in patch.hook().targets(next) {
            if !segment.instructions.contains_key(&target) {
                return Err(TranslationError::TargetNotCode { name, site, target });
            }
            segment.block_starts.insert(target);
        }
        segment.block_starts.insert(site.offset);
        segment.patches.entry(site.offset).or_default().push(*patch);
        Ok(())
    }
}

impl Segment {
    /// Adds every other place execution resumes: the instruction after a call,
    /// an interrupt that may wait, and code after an unconditional transfer.
    fn add_resume_points(&mut self) {
        let mut resume = Vec::new();
        for (&offset, instruction) in &self.instructions {
            match instruction.ir {
                Ir::Interrupt(_) => resume.push(offset),
                _ if ends_block(&instruction.ir) => resume.push(instruction.next()),
                _ => {}
            }
        }
        self.block_starts
            .extend(resume.into_iter().filter(|offset| self.instructions.contains_key(offset)));
    }

    /// Whether any patch replaces the instruction at `offset`.
    pub fn replaced(&self, offset: u16) -> bool {
        self.patches
            .get(&offset)
            .is_some_and(|patches| patches.iter().any(|p| matches!(p.action, Action::Replace(_))))
    }
}

/// Whether an instruction always leaves the current run of instructions.
pub fn ends_block(ir: &Ir) -> bool {
    matches!(
        ir,
        Ir::Jump { cond: None, .. }
            | Ir::JumpIndirect(_)
            | Ir::Call(_)
            | Ir::FarJump(_)
            | Ir::FarJumpIndirect(_)
            | Ir::FarCall(_)
            | Ir::FarCallIndirect(_)
            | Ir::Return { .. }
            | Ir::ReturnFar { .. }
            | Ir::ReturnFromInterrupt
    )
}
