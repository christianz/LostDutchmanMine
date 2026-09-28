//! The settings menu's font metrics.
#![expect(clippy::float_cmp, reason = "text is measured exactly as the menu lays it out")]

use desktop::font::{GLYPHS, glyph, text_width};

#[test]
fn every_printable_character_has_a_glyph() {
    assert_eq!(GLYPHS.len(), usize::from(b'~' - b' ') + 1);
    assert_eq!(glyph(b' ').map(|space| space.advance), Some(27.9688));
    assert_eq!(glyph(b'~').map(|tilde| (tilde.x, tilde.y)), Some((312, 552)));
}

#[test]
fn control_and_non_ascii_bytes_have_no_glyph() {
    for byte in [0, b'\n', 31, 127, 128, 255] {
        assert!(glyph(byte).is_none(), "{byte}");
    }
}

#[test]
fn text_is_as_wide_as_its_advances_at_the_baked_size() {
    assert_eq!(text_width("", 17.0), 0.0);
    assert_eq!(text_width("AB", 88.0), 60.2031 + 60.3750);
    assert_eq!(text_width("A", 44.0), 60.2031 / 2.0);
}

#[test]
fn characters_the_font_lacks_take_no_space() {
    assert_eq!(text_width("A\u{e9}\tB", 88.0), text_width("AB", 88.0));
}
