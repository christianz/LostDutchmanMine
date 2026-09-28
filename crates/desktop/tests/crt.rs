//! The CRT monitor effect: its scanline and phosphor mask, and the highlight glow.

use desktop::pixels::{self, Pixels, crt_glow, crt_mask};
use desktop::settings::Crt;

const WIDTH: i32 = 2880;
const HEIGHT: i32 = 2160;
const BLACK: u32 = 0xff00_0000;
const WHITE: u32 = 0xffff_ffff;

/// A 4K display's full-height 4:3 picture.
fn mask(effect: Crt) -> Vec<u32> {
    let mut output = Vec::new();
    crt_mask(effect, WIDTH, HEIGHT, &mut output);
    output
}

fn at(x: i32, y: i32) -> usize {
    (y * WIDTH + x) as usize
}

/// The sum of a pixel's channels.
fn light(pixel: u32) -> u32 {
    ((pixel >> 16) & 0xff) + ((pixel >> 8) & 0xff) + (pixel & 0xff)
}

fn glow(picture: &[u32]) -> Box<Pixels> {
    let mut output = pixels::filled(0);
    crt_glow(picture, 320, 200, &mut output);
    output
}

#[test]
fn crt_off_is_an_identity_mask() {
    let off = mask(Crt::Off);
    assert_eq!(off.len(), (WIDTH * HEIGHT) as usize);
    assert!(off.iter().all(|&pixel| pixel == WHITE));
}

#[test]
fn strong_is_stronger_than_subtle() {
    let centre = at(1440, 1080);
    assert!(light(mask(Crt::Subtle)[centre]) > light(mask(Crt::Strong)[centre]));
}

#[test]
fn edges_darken_gently() {
    let strong = mask(Crt::Strong);
    assert!(light(strong[at(1440, 1080)]) > light(strong[at(0, 1080)]));
}

#[test]
fn the_mask_has_coloured_phosphors() {
    let centre = mask(Crt::Strong)[at(1440, 1080)];
    assert_ne!((centre >> 16) & 0xff, centre & 0xff);
}

#[test]
fn the_mask_has_scanlines() {
    let strong = mask(Crt::Strong);
    assert_ne!(light(strong[at(1440, 1080)]), light(strong[at(1440, 1085)]));
}

#[test]
fn the_mask_is_opaque() {
    assert!(mask(Crt::Subtle).iter().all(|&pixel| pixel >> 24 == 0xff));
}

#[test]
fn an_empty_crt_viewport_is_safe() {
    let mut output = vec![WHITE; 3];
    crt_mask(Crt::Strong, 0, HEIGHT, &mut output);
    assert!(output.is_empty());
    crt_mask(Crt::Off, WIDTH, -1, &mut output);
    assert!(output.is_empty());
}

#[test]
fn black_areas_do_not_glow() {
    assert!(glow(&vec![BLACK; 64_000]).iter().all(|&pixel| pixel == BLACK));
}

#[test]
fn highlights_glow_softly() {
    let mut picture = vec![BLACK; 64_000];
    picture[100 * 320 + 160] = WHITE;
    let glow = glow(&picture);
    let (centre, beside) = (light(glow[100 * 320 + 160]), light(glow[100 * 320 + 161]));
    assert!(centre > beside && beside > 0, "centre {centre}, beside {beside}");
}

#[test]
fn glow_stays_local_and_preserves_the_source() {
    let mut picture = vec![BLACK; 64_000];
    picture[100 * 320 + 160] = WHITE;
    let source = picture.clone();
    assert_eq!(glow(&picture)[100 * 320 + 164], BLACK);
    assert_eq!(picture, source);
}

#[test]
fn an_enlarged_picture_glows_like_its_frame() {
    let mut frame = vec![BLACK; 64_000];
    frame[100 * 320 + 160] = WHITE;
    let enlarged: Vec<u32> =
        (0..640 * 400).map(|at| frame[(at / 640 / 2) * 320 + (at % 640) / 2]).collect();
    let mut output = pixels::filled(0);
    crt_glow(&enlarged, 640, 400, &mut output);
    assert_eq!(output, glow(&frame));
}

#[test]
#[should_panic(expected = "CRT glow")]
fn a_glow_source_of_another_size_is_refused() {
    let mut output = pixels::filled(0);
    crt_glow(&vec![BLACK; 480 * 300], 480, 300, &mut output);
}
