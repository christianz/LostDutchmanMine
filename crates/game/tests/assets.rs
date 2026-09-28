//! The asset stream decoder on synthetic streams.

use game::{AssetError, decode_asset};

/// Encodes `codes` MSB-first behind the stream header; code 256 widens every
/// later code by one bit, as in the original encoder.
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
    let [packed_low, packed_high] = (packed.len() as u16).to_le_bytes();
    let [declared_low, declared_high] = declared.to_le_bytes();
    let mut data = vec![1, 0x9d, packed_low, packed_high, declared_low, declared_high];
    data.extend(packed);
    data
}

#[test]
fn literal_codes_are_bytes() {
    assert_eq!(decode_asset(&stream(&[65, 66], 2)), Ok(b"AB".to_vec()));
}

#[test]
fn each_pair_of_tokens_defines_a_dictionary_entry() {
    // Tokens 0 and 2 start entries: entry 0 is "AB", from token 0 to token 2.
    assert_eq!(decode_asset(&stream(&[65, 66, 67, 257], 5)), Ok(b"ABCAB".to_vec()));
}

#[test]
fn code_256_widens_the_codes_that_follow() {
    assert_eq!(decode_asset(&stream(&[65, 256, 66, 67], 3)), Ok(b"ABC".to_vec()));
}

#[test]
fn padding_beyond_the_declared_size_is_kept() {
    assert_eq!(decode_asset(&stream(&[65, 66, 67], 2)), Ok(b"ABC".to_vec()));
}

#[test]
fn a_stream_shorter_than_declared_is_rejected() {
    assert_eq!(decode_asset(&stream(&[65], 2)), Err(AssetError::Short));
}

#[test]
fn a_foreign_header_is_rejected() {
    let mut data = stream(&[65], 1);
    data[1] = 0x9e;
    assert_eq!(decode_asset(&data), Err(AssetError::Invalid));
}

#[test]
fn a_reference_to_an_unfinished_entry_is_rejected() {
    assert_eq!(decode_asset(&stream(&[65, 257], 2)), Err(AssetError::Invalid));
}
