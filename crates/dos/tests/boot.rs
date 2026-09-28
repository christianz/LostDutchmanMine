//! The DOS loader and the environment it leaves for the program.

use dos::{Image, boot};
use machine::{Address, LOAD_SEGMENT, Machine};

#[test]
fn loads_relocates_and_sets_up_the_program_environment() {
    let mut bytes = vec![0; 0x40];
    bytes[0x10..0x12].copy_from_slice(&0x0002_u16.to_le_bytes()); // A segment word.
    let image = Image {
        bytes: &bytes,
        relocations: &[0x10],
        entry: Address::new(1, 4),
        stack: Address::new(3, 0x80),
    };
    let mut m = Machine::new();
    boot(&mut m, &image).expect("small image");

    assert_eq!(m.memory.read16(LOAD_SEGMENT, 0x10), 0x1002, "relocated by the load segment");
    let r = &m.regs;
    assert_eq!((r.cs, r.ip, r.ss, r.sp), (0x1001, 4, 0x1003, 0x80));
    assert_eq!((r.ds, r.es), (0x0ff0, 0x0ff0), "DS and ES address the PSP");
    assert_eq!(m.memory.read16(0x0ff0, 0), 0x20cd, "PSP starts with INT 20h");
    assert_eq!(m.memory.read16(0x0ff0, 0x2c), 0x0f00, "environment segment");
    let environment = &m.memory.as_bytes()[0xf000..0xf009];
    assert_eq!(environment, b"PATH=C:\\\0");
    assert_eq!(
        (m.memory.read16(0, 0x20), m.memory.read16(0, 0x22)),
        (0x20, 0xf000),
        "IVT points into the BIOS"
    );
    assert_eq!(m.memory.read16(0x40, 0x13), 640, "640 KB conventional memory");
    assert_eq!(m.vga.palette[0x3f], 0xffff_ffff);
    assert_eq!(m.vga.palette[6], 0xffaa_aa00, "brown without the VGA red boost");
    assert_eq!(m.vga.attributes[6], 20, "the attribute controller maps brown to 20");
}

#[test]
fn images_that_do_not_fit_below_the_video_area_are_refused() {
    let bytes = vec![0; 0x9_0001];
    let image = Image {
        bytes: &bytes,
        relocations: &[],
        entry: Address::new(0, 0),
        stack: Address::new(0, 0),
    };
    assert!(boot(&mut Machine::new(), &image).is_err());
}
