//! The readable asset decoder on synthetic streams, called as the game calls
//! it: stream at DS:SI, output at ES:DI, a near return address on the stack.

use machine::{Flag, Machine};

use super::assets::decode;
use crate::assets::decode_asset;

/// Where the stream, the output and the stack live.
const STREAM: (u16, u16) = (12_288, 256);
const OUTPUT: (u16, u16) = (20_480, 0);
const RETURN: u16 = 40_000;

/// Encodes `codes` MSB-first behind the stream header; 256 widens later codes.
fn stream(codes: &[u16], declared: u16) -> Vec<u8> {
    let mut bits = Vec::new();
    let mut width = 9;
    for &code in codes {
        bits.extend((0..width).rev().map(|bit| (code >> bit) & 1 == 1));
        if code == 256 {
            width += 1;
        }
    }
    let packed: Vec<u8> = bits
        .chunks(8)
        .map(|byte| {
            byte.iter().enumerate().fold(0, |acc, (i, &on)| acc | (u8::from(on) << (7 - i)))
        })
        .collect();
    let mut data = vec![1, 0x9d];
    data.extend((packed.len() as u16).to_le_bytes());
    data.extend(declared.to_le_bytes());
    data.extend(packed);
    data
}

fn called_with(data: &[u8]) -> Machine {
    let mut m = Machine::new();
    (m.regs.ds, m.regs.si, m.regs.es, m.regs.di) = (STREAM.0, STREAM.1, OUTPUT.0, OUTPUT.1);
    (m.regs.ss, m.regs.sp, m.regs.bx, m.regs.cx, m.regs.dx, m.regs.bp) = (36_864, 4096, 1, 2, 3, 4);
    for (i, &byte) in (0..).zip(data) {
        m.memory.write8(STREAM.0, STREAM.1 + i, byte);
    }
    m.push(RETURN);
    m
}

fn output(m: &Machine, length: u16) -> Vec<u8> {
    (0..length).map(|i| m.memory.read8(OUTPUT.0, OUTPUT.1 + i)).collect()
}

#[test]
fn a_stream_decodes_as_the_native_decoder_decodes_it() {
    for codes in [&[65, 66][..], &[65, 66, 67, 257], &[65, 256, 66, 67], &[72, 73, 257, 74, 258]] {
        let data = stream(codes, 0);
        let expected = decode_asset(&data).expect("a valid stream");
        let mut m = called_with(&data);
        assert!(decode(&mut m), "decoded");
        assert_eq!(m.regs.ax, expected.len() as u16, "{codes:?}: AX is the end of the output");
        assert_eq!(output(&m, m.regs.ax), expected, "{codes:?}");
        assert!(!m.flag(Flag::Carry), "{codes:?}: CF clear");
    }
}

#[test]
fn it_returns_to_its_caller_with_every_other_register_kept() {
    let mut m = called_with(&stream(&[65, 66, 67, 257], 5));
    assert!(decode(&mut m), "decoded");
    assert_eq!((m.regs.ip, m.regs.sp), (RETURN, 4096));
    let kept =
        (m.regs.bx, m.regs.cx, m.regs.dx, m.regs.bp, m.regs.si, m.regs.di, m.regs.ds, m.regs.es);
    assert_eq!(kept, (1, 2, 3, 4, STREAM.1, OUTPUT.1, STREAM.0, OUTPUT.0));
    assert!(m.flag(Flag::Zero) && !m.flag(Flag::Sign), "flags as its last comparison left them");
}

#[test]
fn the_dictionary_is_kept_in_the_streams_own_memory() {
    let mut m = called_with(&stream(&[65, 66, 67, 257], 5));
    assert!(decode(&mut m), "decoded");
    // Entries start ten bytes before the stream: each is where a pair of
    // tokens began in the output.
    let entry = |n: u16| m.memory.read16(STREAM.0, STREAM.1 - 10 + n * 2);
    assert_eq!((entry(0), entry(1)), (0, 2));
}

#[test]
fn a_foreign_stream_is_refused_with_carry_set() {
    let mut data = stream(&[65], 1);
    data[1] = 0x9e;
    let mut m = called_with(&data);
    assert!(decode(&mut m), "decoded");
    assert!(m.flag(Flag::Carry));
    assert_eq!((m.regs.cx, m.regs.ax, m.regs.ip), (0, u16::from_le_bytes([1, 0x9e]), RETURN));
}

#[test]
fn a_set_direction_flag_leaves_the_call_to_the_original() {
    let mut m = called_with(&stream(&[65], 1));
    m.set_flag(Flag::Direction, true);
    let before = m.clone();
    assert!(!decode(&mut m));
    assert_eq!(m.state_hash(), before.state_hash());
}
