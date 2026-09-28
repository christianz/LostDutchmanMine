//! Savestates of DOS: queued input, the mouse, the clock, the video mode and
//! open files by folder, name and position.

use std::fs;
use std::path::PathBuf;

use dos::Dos;
use machine::Machine;
use machine::state::{Persist, Reader, Writer};

struct Folders {
    root: PathBuf,
}

impl Folders {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("ldm-dos-state-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("game")).expect("data folder");
        fs::create_dir_all(root.join("saves")).expect("saves folder");
        fs::write(root.join("game/DATA.BIN"), (0..100).collect::<Vec<u8>>()).expect("data file");
        Folders { root }
    }

    fn dos(&self) -> Dos {
        Dos::new(self.root.join("game"), self.root.join("saves"))
    }
}

impl Drop for Folders {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn service(dos: &mut Dos, m: &mut Machine, ax: u16) {
    m.regs.ax = ax;
    dos.interrupt(m, 0x21).expect("a supported service");
}

/// Opens DS:0100 for reading and returns its handle.
fn open(dos: &mut Dos, m: &mut Machine, name: &str, mode: u16) -> u16 {
    m.regs.ds = 0x2000;
    for (i, byte) in name.bytes().chain([0]).enumerate() {
        m.memory.write8(0x2000, 0x100 + i as u16, byte);
    }
    m.regs.dx = 0x100;
    service(dos, m, 0x3d00 | mode);
    m.regs.ax
}

/// Reads `count` bytes from `handle` into DS:0200.
fn read(dos: &mut Dos, m: &mut Machine, handle: u16, count: u16) -> Vec<u8> {
    (m.regs.bx, m.regs.cx, m.regs.dx) = (handle, count, 0x200);
    service(dos, m, 0x3f00);
    (0..m.regs.ax).map(|i| m.memory.read8(0x2000, 0x200 + i)).collect()
}

fn round_trip(dos: &Dos, into: &mut Dos) {
    let mut writer = Writer::new();
    dos.save(&mut writer);
    let state = writer.finish();
    let mut reader = Reader::new(&state).expect("a state");
    into.restore(&mut reader).expect("a valid state");
    reader.finish().expect("nothing left over");
}

#[test]
fn an_open_file_continues_where_it_was() {
    let folders = Folders::new("files");
    let (mut dos, mut m) = (folders.dos(), Machine::new());
    let handle = open(&mut dos, &mut m, "data.bin", 0);
    assert_eq!(read(&mut dos, &mut m, handle, 10), (0..10).collect::<Vec<u8>>());
    let mut copy = folders.dos();
    round_trip(&dos, &mut copy);
    assert_eq!(read(&mut copy, &mut m, handle, 10), (10..20).collect::<Vec<u8>>());
    assert_eq!(
        read(&mut dos, &mut m, handle, 10),
        (10..20).collect::<Vec<u8>>(),
        "the original goes on"
    );
}

#[test]
fn a_file_open_for_writing_stays_in_the_saves_folder() {
    let folders = Folders::new("writes");
    let (mut dos, mut m) = (folders.dos(), Machine::new());
    let handle = open(&mut dos, &mut m, "DATA.BIN", 2);
    read(&mut dos, &mut m, handle, 50);
    let mut copy = folders.dos();
    round_trip(&dos, &mut copy);
    (m.regs.bx, m.regs.cx, m.regs.dx) = (handle, 1, 0x300);
    m.memory.write8(0x2000, 0x300, 0xee);
    service(&mut copy, &mut m, 0x4000);
    drop(copy);
    let saved = fs::read(folders.root.join("saves/DATA.BIN")).expect("the saves copy");
    assert_eq!(saved[50], 0xee, "written at the restored position");
    assert_eq!(fs::read(folders.root.join("game/DATA.BIN")).expect("the original")[50], 50);
}

#[test]
fn input_video_and_the_clock_survive() {
    let folders = Folders::new("input");
    let mut dos = folders.dos();
    dos.keyboard.push(0x1e61);
    dos.keyboard.push(0x3920);
    dos.keyboard.movement_aliases = true;
    dos.mouse.input.set_time(1500);
    dos.mouse.input.move_to(10, 20);
    dos.mouse.input.buttons(1);
    dos.mouse.input.buttons(0);
    dos.mouse.visibility = 0;
    dos.mouse.hotspot = (3, 4);
    dos.mouse.shape[7] = 0x1234;
    dos.video.mode = 0x13;
    dos.clock.emulated_ms = 12_345;
    dos.clock.ticks = 77;
    dos.clock.base = 1_800_000_000;
    let mut copy = folders.dos();
    round_trip(&dos, &mut copy);
    assert_eq!(copy.keyboard.iter().collect::<Vec<_>>(), [0x1e61, 0x3920]);
    assert!(copy.keyboard.movement_aliases);
    for mouse in [&mut dos.mouse, &mut copy.mouse] {
        mouse.input.poll();
    }
    assert_eq!(copy.mouse.input.sample(), dos.mouse.input.sample(), "the queued click survives");
    assert_eq!(
        (copy.mouse.visibility, copy.mouse.hotspot, copy.mouse.shape[7]),
        (0, (3, 4), 0x1234)
    );
    assert_eq!(copy.video, dos.video);
    assert_eq!(
        (copy.clock.emulated_ms, copy.clock.ticks, copy.clock.base),
        (12_345, 77, 1_800_000_000)
    );
}
