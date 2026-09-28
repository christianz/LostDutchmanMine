//! The game's packed-asset decoder (1265:1250), as readable Rust.
//!
//! A stream is the two bytes `01 9D`, its packed and unpacked lengths, then
//! codes most significant bit first, nine bits wide to begin with. Code 256
//! widens every later code by one bit. A code below 256 is a byte. Code
//! `257 + n` repeats dictionary entry `n`: the output from where the `n`th pair
//! of tokens began to where the next pair began.
//!
//! The original keeps its dictionary in the stream's own memory, starting ten
//! bytes before it, so each entry overwrites input it has already read, and its
//! state in the stream's segment at [`DECODER_CODE_WIDTH`] and on. This keeps
//! both where the original does, and copies as it does, so memory afterwards
//! is the original's byte for byte, whatever the stream holds.

use machine::{AluOp, Flag, Machine, Width};

use crate::symbols::{DECODER_BITS_LEFT, DECODER_BYTE, DECODER_BYTES_LEFT, DECODER_CODE_WIDTH};

/// The stream's first word: `01 9D`.
const MAGIC: u16 = u16::from_le_bytes([0x01, 0x9d]);
/// Codes below this are bytes.
const WIDEN: u16 = 256;
const FIRST_ENTRY: u16 = 257;
/// The dictionary starts this far before the stream.
const DICTIONARY_BEFORE_STREAM: u16 = 10;
/// The codes start after the six-byte header.
const HEADER: u16 = 6;
const FIRST_WIDTH: u16 = 9;

/// The decoder's view of memory: its stream segment, where its state lives.
struct Decoder<'a> {
    m: &'a mut Machine,
    /// The stream's segment, DS at the call.
    stream: u16,
    /// The next packed byte.
    input: u16,
    /// Where the next output byte goes, in ES.
    out: u16,
}

impl Decoder<'_> {
    fn word(&self, offset: u16) -> u16 {
        self.m.memory.read16(self.stream, offset)
    }

    fn set_word(&mut self, offset: u16, value: u16) {
        self.m.memory.write16(self.stream, offset, value);
    }

    /// Reads the next code, one bit at a time. When the packed bytes run out
    /// mid-code, the code read so far is returned and the stream is over.
    fn code(&mut self) -> u16 {
        let mut code = 0u16;
        let mut bits_left = self.word(DECODER_BITS_LEFT);
        let mut byte = self.m.memory.read8(self.stream, DECODER_BYTE);
        let mut width = self.word(DECODER_CODE_WIDTH);
        loop {
            bits_left = bits_left.wrapping_sub(1);
            if (bits_left as i16) < 0 {
                byte = self.m.memory.read8(self.stream, self.input);
                self.input = self.input.wrapping_add(1);
                bits_left = 7;
                let bytes_left = self.word(DECODER_BYTES_LEFT).wrapping_sub(1);
                self.set_word(DECODER_BYTES_LEFT, bytes_left);
                if bytes_left == 0 {
                    break;
                }
            }
            code = code << 1 | u16::from(byte >> 7);
            byte <<= 1;
            width = width.wrapping_sub(1);
            if width == 0 {
                break;
            }
        }
        self.set_word(DECODER_BITS_LEFT, bits_left);
        self.m.memory.write8(self.stream, DECODER_BYTE, byte);
        code
    }

    /// Reads codes, widening at each 256, until one that is not.
    fn token(&mut self) -> u16 {
        loop {
            let code = self.code();
            if code != WIDEN {
                return code;
            }
            let width = self.word(DECODER_CODE_WIDTH).wrapping_add(1);
            self.set_word(DECODER_CODE_WIDTH, width);
        }
    }

    fn stream_over(&self) -> bool {
        self.word(DECODER_BYTES_LEFT) == 0
    }

    /// Writes a token's bytes: a byte, or a dictionary entry copied from
    /// earlier output, an odd byte first, then words, as the original does.
    fn emit(&mut self, token: u16, dictionary: u16) {
        let es = self.m.regs.es;
        // A signed comparison, as in the original: codes from 8000h are bytes too.
        if (token as i16) <= 255 {
            self.m.memory.write8(es, self.out, token as u8);
            self.out = self.out.wrapping_add(1);
            return;
        }
        let entry = dictionary.wrapping_add(token.wrapping_sub(FIRST_ENTRY).wrapping_mul(2));
        let (start, end) = (self.word(entry), self.word(entry.wrapping_add(2)));
        let mut length = end.wrapping_sub(start);
        let mut from = start;
        if length & 1 != 0 {
            let byte = self.m.memory.read8(es, from);
            self.m.memory.write8(es, self.out, byte);
            (from, self.out) = (from.wrapping_add(1), self.out.wrapping_add(1));
        }
        length >>= 1;
        for _ in 0..length {
            let word = self.m.memory.read16(es, from);
            self.m.memory.write16(es, self.out, word);
            (from, self.out) = (from.wrapping_add(2), self.out.wrapping_add(2));
        }
    }
}

/// Decodes the stream at DS:SI to ES:DI and returns to the caller: AX is the
/// end of the output and CF clear, or for a foreign stream AX is its first
/// word, CX zero and CF set. Every other register is kept, and the flags are
/// those the original's last comparison leaves.
///
/// Declines, changing nothing, when the direction flag is set: the game's C
/// runtime keeps it clear, and the original would then copy backwards.
pub(crate) fn decode(m: &mut Machine) -> bool {
    if m.flag(Flag::Direction) {
        return false;
    }
    let (stream, start) = (m.regs.ds, m.regs.si);
    let first = m.memory.read16(stream, start);
    if first != MAGIC {
        m.regs.ax = first;
        m.alu(AluOp::Sub, first, MAGIC, Width::Word);
        m.regs.cx = 0;
        m.set_flag(Flag::Carry, true);
        m.regs.ip = m.pop();
        return true;
    }
    let dictionary = start.wrapping_sub(DICTIONARY_BEFORE_STREAM);
    let mut decoder = Decoder { stream, input: start.wrapping_add(HEADER), out: m.regs.di, m };
    let packed = decoder.word(start.wrapping_add(2));
    decoder.set_word(DECODER_BYTES_LEFT, packed.wrapping_add(1));
    decoder.set_word(DECODER_BITS_LEFT, 0);
    decoder.set_word(DECODER_CODE_WIDTH, FIRST_WIDTH);
    let mut next_entry = dictionary;
    'pairs: loop {
        let out = decoder.out;
        decoder.set_word(next_entry, out);
        next_entry = next_entry.wrapping_add(2);
        for _ in 0..2 {
            let token = decoder.token();
            if decoder.stream_over() {
                break 'pairs;
            }
            decoder.emit(token, dictionary);
        }
    }
    let end = decoder.out;
    m.regs.ax = end;
    // The original's last comparison found no packed bytes left.
    m.alu(AluOp::Sub, 0, 0, Width::Word);
    m.set_flag(Flag::Carry, false);
    m.regs.ip = m.pop();
    true
}
