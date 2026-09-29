//! Frames and reports composed from a synthetic machine.

use std::path::PathBuf;

use game::symbols::{
    CGA_FRAMEBUFFER, DATA_SEGMENT, POSITION_X, SCENE_MAP, VGA_FRAMEBUFFER, VIDEO_MODE_CGA,
    VIDEO_MODE_VGA,
};
use game::{Frame, Game};
use machine::{LOAD_SEGMENT, Machine};

const BLACK: u32 = 0xff00_0000;
const WHITE: u32 = 0xffff_ffff;

fn setup(qol: bool) -> (Machine, Game) {
    (Machine::new(), Game::new(PathBuf::from("data"), PathBuf::from("saves"), qol))
}

fn at(frame: &Frame, x: usize, y: usize) -> u32 {
    frame.pixels()[y * Frame::WIDTH + x]
}

#[test]
fn text_modes_show_black() {
    let (m, g) = setup(true);
    assert!(g.frame(&m).pixels().iter().all(|&pixel| pixel == BLACK));
}

#[test]
fn vga_frames_map_the_framebuffer_through_the_palette() {
    let (mut m, mut g) = setup(true);
    g.dos.video.mode = VIDEO_MODE_VGA;
    m.vga.palette[7] = 0xff12_3456;
    m.memory.as_bytes_mut()[VGA_FRAMEBUFFER + 321] = 7;
    let frame = g.frame(&m);
    assert_eq!(at(&frame, 1, 1), 0xff12_3456);
    assert_eq!(at(&frame, 0, 0), m.vga.palette[0]);
}

#[test]
fn cga_frames_decode_two_bit_pixels_from_interleaved_rows() {
    let (mut m, mut g) = setup(true);
    g.dos.video.mode = VIDEO_MODE_CGA;
    let memory = m.memory.as_bytes_mut();
    memory[CGA_FRAMEBUFFER] = 0b00_01_10_11;
    memory[CGA_FRAMEBUFFER + 8192] = 0b11_00_00_00;
    let frame = g.frame(&m);
    let row: Vec<u32> = (0..4).map(|x| at(&frame, x, 0)).collect();
    assert_eq!(row, [BLACK, 0xff55_ffff, 0xffff_55ff, WHITE]);
    assert_eq!(at(&frame, 0, 1), WHITE, "odd rows come from the second bank");
}

#[test]
fn the_classic_cursor_is_the_games_own_mask_at_its_hotspot() {
    let (mut m, mut g) = setup(false);
    g.dos.video.mode = VIDEO_MODE_VGA;
    g.dos.mouse.custom_cursor = true;
    g.dos.mouse.visibility = 0;
    g.dos.mouse.hotspot = (2, 1);
    g.dos.mouse.shape = [u16::MAX; 32];
    g.dos.mouse.shape[16] = 1 << 15;
    g.dos.mouse.input.move_to(100, 50);
    m.vga.palette[0] = BLACK;
    let frame = g.frame(&m);
    assert_eq!(at(&frame, 98, 49), WHITE, "the screen mask inverts at the hotspot's corner");
    assert_eq!(at(&frame, 99, 49), BLACK);
}

#[test]
fn the_qol_cursor_is_an_outlined_arrow_tipped_at_the_pointer() {
    let (mut m, mut g) = setup(true);
    g.dos.video.mode = VIDEO_MODE_VGA;
    g.dos.mouse.custom_cursor = true;
    g.dos.mouse.visibility = 0;
    g.dos.mouse.input.move_to(100, 50);
    m.vga.palette[0] = 0xff20_2020;
    let frame = g.frame(&m);
    assert_eq!(at(&frame, 100, 50), BLACK);
    assert_eq!(at(&frame, 101, 52), WHITE);
    assert_eq!(at(&frame, 101, 50), 0xff20_2020, "outside the arrow");
}

#[test]
fn a_hidden_pointer_is_not_drawn() {
    let (mut m, mut g) = setup(true);
    g.dos.video.mode = VIDEO_MODE_VGA;
    g.dos.mouse.custom_cursor = true;
    g.dos.mouse.visibility = -1;
    m.vga.palette[0] = 0xff20_2020;
    assert!(g.frame(&m).pixels().iter().all(|&pixel| pixel == 0xff20_2020));
}

#[test]
fn reports_read_the_data_segment_between_runs() {
    let (mut m, g) = setup(true);
    let data = DATA_SEGMENT + LOAD_SEGMENT;
    m.memory.write16(data, POSITION_X.0, 123);
    m.memory.write16(data, SCENE_MAP.0, 1);
    m.regs.ds = 0;
    let report = g.report(&m);
    assert_eq!((report.x, report.map_view, report.qol), (123, true, true));
}
