//! Placing the picture, mapping the mouse onto it, colour grading and enlarging.

use desktop::pixels::{
    self, Pixels, Rect, colour_pixel, display_pixels, nearest_picture_point, picture_point,
    picture_rect,
};
use desktop::settings::{Colour, DisplaySettings, Scaling};

const GREY: u32 = 0xff77_7777;
const WHITE: u32 = 0xffff_ffff;
const BLACK: u32 = 0xff00_0000;

/// A frame of 64,000 different colours.
fn gradient() -> Box<Pixels> {
    let mut frame = pixels::filled(0);
    for (i, pixel) in frame.iter_mut().enumerate() {
        *pixel = BLACK | (i as u32 * 997);
    }
    frame
}

fn scaled(scaling: Scaling) -> DisplaySettings {
    DisplaySettings { scaling, ..DisplaySettings::default() }
}

#[test]
fn a_4k_screen_shows_the_picture_at_4_by_3() {
    assert_eq!(picture_rect(3840, 2160, 100), Rect { x: 480, y: 0, w: 2880, h: 2160 });
}

#[test]
fn a_smaller_picture_is_centred() {
    assert_eq!(picture_rect(3840, 2160, 85), Rect { x: 696, y: 162, w: 2448, h: 1836 });
}

#[test]
fn an_empty_display_has_an_empty_picture() {
    assert_eq!(picture_rect(0, 2160, 100), Rect::default());
    assert_eq!(picture_rect(3840, -1, 100), Rect::default());
}

#[test]
fn the_mouse_maps_to_the_original_coordinates() {
    let picture = picture_rect(3840, 2160, 100);
    assert_eq!(picture_point(picture, 1920, 1080), Some((160, 100)), "centre");
    let bottom_right = (picture.x + picture.w - 1, picture.y + picture.h - 1);
    assert_eq!(picture_point(picture, bottom_right.0, bottom_right.1), Some((319, 199)));
}

#[test]
fn letterbox_clicks_are_rejected() {
    let picture = picture_rect(3840, 2160, 100);
    assert_eq!(picture_point(picture, 10, 100), None);
    assert_eq!(
        picture_point(picture, picture.x + picture.w, 100),
        None,
        "just past the right edge"
    );
}

#[test]
fn pointer_motion_over_the_letterbox_tracks_the_nearest_edge() {
    let picture = picture_rect(3840, 2160, 100);
    assert_eq!(nearest_picture_point(picture, 10, 1080), Some((0, 100)));
    assert_eq!(nearest_picture_point(picture, 3839, 1080), Some((319, 100)));
    assert_eq!(nearest_picture_point(picture, 1920, 1080), picture_point(picture, 1920, 1080));
}

#[test]
fn an_empty_viewport_is_safe() {
    assert_eq!(picture_point(Rect::default(), 0, 0), None);
    assert_eq!(nearest_picture_point(Rect::default(), 0, 0), None);
}

#[test]
fn crisp_preserves_every_pixel() {
    let frame = gradient();
    let mut output = Vec::new();
    assert_eq!(display_pixels(&frame, &scaled(Scaling::Crisp), &mut output), (320, 200));
    assert_eq!(output[..], frame[..]);
}

#[test]
fn soft_doubles_each_source_texel_and_leaves_the_source_alone() {
    let frame = gradient();
    let before = frame.clone();
    let mut output = Vec::new();
    assert_eq!(display_pixels(&frame, &scaled(Scaling::Soft), &mut output), (640, 400));
    assert_eq!(output.len(), 640 * 400);
    assert_eq!(output[0], frame[0]);
    assert_eq!(output[1], output[0]);
    assert_eq!(output[640], output[0]);
    assert_eq!(output[2 * 640 + 2], frame[320 + 1]);
    assert_eq!(frame, before, "presentation never modifies the source artwork");
}

#[test]
fn a_reused_output_buffer_is_resized_for_each_filter() {
    let frame = gradient();
    let mut output = vec![WHITE; 7];
    display_pixels(&frame, &scaled(Scaling::PixelArt), &mut output);
    assert_eq!(output.len(), 640 * 400);
    display_pixels(&frame, &scaled(Scaling::Crisp), &mut output);
    assert_eq!(output.len(), 320 * 200);
}

#[test]
fn pixel_art_reconstructs_diagonals() {
    let mut frame = pixels::filled(GREY);
    frame[99 * 320 + 100] = WHITE;
    frame[100 * 320 + 99] = WHITE;
    frame[101 * 320 + 100] = BLACK;
    frame[100 * 320 + 101] = BLACK;
    let mut output = Vec::new();
    display_pixels(&frame, &scaled(Scaling::PixelArt), &mut output);
    let at = 200 * 640 + 200;
    assert_eq!([output[at], output[at + 1]], [WHITE, GREY]);
    assert_eq!([output[at + 640], output[at + 641]], [GREY, BLACK]);
}

#[test]
fn pixel_art_rounds_diagonals_in_graded_colours() {
    let mut frame = pixels::filled(GREY);
    frame[99 * 320 + 100] = WHITE;
    frame[100 * 320 + 99] = WHITE;
    let settings = DisplaySettings { colour: Colour::Warm, ..scaled(Scaling::PixelArt) };
    let mut output = Vec::new();
    display_pixels(&frame, &settings, &mut output);
    let at = 200 * 640 + 200;
    assert_eq!(output[at], colour_pixel(WHITE, &settings));
    assert_eq!(output[at + 641], colour_pixel(GREY, &settings));
}

#[test]
fn every_filter_grades_every_pixel() {
    let frame = gradient();
    for scaling in Scaling::ALL {
        let settings = DisplaySettings {
            colour: Colour::Vivid,
            brightness: 90,
            scaling,
            ..DisplaySettings::default()
        };
        let mut output = Vec::new();
        let (width, _) = display_pixels(&frame, &settings, &mut output);
        let factor = (width / 320) as usize;
        let expected = colour_pixel(frame[123 * 320 + 45], &settings);
        assert_eq!(output[123 * factor * width as usize + 45 * factor], expected, "{scaling:?}");
    }
}

#[test]
fn colour_grading_preserves_alpha() {
    for colour in Colour::ALL {
        let settings = DisplaySettings { colour, brightness: 120, ..DisplaySettings::default() };
        assert_eq!(colour_pixel(0xfffe_fefe, &settings) >> 24, 0xff, "{colour:?}");
    }
}

#[test]
fn the_original_palette_is_unchanged() {
    assert_eq!(colour_pixel(0xff34_79be, &DisplaySettings::default()), 0xff34_79be);
}

#[test]
fn warm_reduces_blue_relative_to_red() {
    let settings = DisplaySettings { colour: Colour::Warm, ..DisplaySettings::default() };
    let warm = colour_pixel(0xff80_8080, &settings);
    let (red, blue) = ((warm >> 16) & 0xff, warm & 0xff);
    assert!(red > blue, "red {red} > blue {blue}");
}
