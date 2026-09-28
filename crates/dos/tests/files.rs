//! DOS file services: case-insensitive names, and writes that never touch the
//! original game directory.

use std::fs;
use std::path::PathBuf;

use dos::Dos;
use machine::{Flag, Machine};

struct Folders {
    root: PathBuf,
}

impl Folders {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("ldm-dos-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("game/LDMG")).expect("data folder");
        fs::write(root.join("game/LDMG/PANL.ZZZ"), b"panel").expect("data file");
        fs::write(root.join("game/LDMSAVE1.SAV"), b"original save").expect("data file");
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

/// Puts a NUL-terminated name at DS:0100.
fn name(m: &mut Machine, text: &str) {
    m.regs.ds = 0x2000;
    for (i, byte) in text.bytes().chain([0]).enumerate() {
        m.memory.write8(0x2000, 0x100 + i as u16, byte);
    }
    m.regs.dx = 0x100;
}

fn service(dos: &mut Dos, m: &mut Machine, ax: u16) {
    m.regs.ax = ax;
    dos.interrupt(m, 0x21).expect("supported service");
}

fn read(dos: &mut Dos, m: &mut Machine, handle: u16, count: u16) -> Vec<u8> {
    (m.regs.bx, m.regs.cx, m.regs.dx) = (handle, count, 0x300);
    service(dos, m, 0x3f00);
    (0..m.regs.ax).map(|i| m.memory.read8(0x2000, 0x300 + i)).collect()
}

#[test]
fn names_resolve_case_insensitively_with_drive_and_backslashes() {
    let folders = Folders::new("names");
    let (mut dos, mut m) = (folders.dos(), Machine::new());
    name(&mut m, "C:\\ldmg\\panl.zzz");
    service(&mut dos, &mut m, 0x3d00);
    assert!(!m.flag(Flag::Carry));
    assert_eq!(m.regs.ax, 5, "handles start at 5");
    assert_eq!(read(&mut dos, &mut m, 5, 100), b"panel");
}

#[test]
fn missing_files_and_unknown_handles_report_dos_errors() {
    let folders = Folders::new("errors");
    let (mut dos, mut m) = (folders.dos(), Machine::new());
    name(&mut m, "NOSUCH.DAT");
    service(&mut dos, &mut m, 0x3d00);
    assert!(m.flag(Flag::Carry) && m.regs.ax == 2, "file not found");
    m.regs.bx = 9;
    service(&mut dos, &mut m, 0x3e00);
    assert!(m.flag(Flag::Carry) && m.regs.ax == 6, "invalid handle");
}

#[test]
fn opening_an_original_file_for_writing_copies_it_first() {
    let folders = Folders::new("copy");
    let (mut dos, mut m) = (folders.dos(), Machine::new());
    name(&mut m, "LDMSAVE1.SAV");
    service(&mut dos, &mut m, 0x3d02);
    let handle = m.regs.ax;
    for (i, byte) in b"NEW".iter().enumerate() {
        m.memory.write8(0x2000, 0x200 + i as u16, *byte);
    }
    (m.regs.bx, m.regs.cx, m.regs.dx) = (handle, 3, 0x200);
    service(&mut dos, &mut m, 0x4000);
    assert_eq!(m.regs.ax, 3);
    service(&mut dos, &mut m, 0x3e00);
    assert_eq!(fs::read(folders.root.join("game/LDMSAVE1.SAV")).unwrap(), b"original save");
    assert_eq!(fs::read(folders.root.join("saves/LDMSAVE1.SAV")).unwrap(), b"NEWginal save");
}

#[test]
fn created_files_seek_and_read_back() {
    let folders = Folders::new("seek");
    let (mut dos, mut m) = (folders.dos(), Machine::new());
    name(&mut m, "ldmg\\ldm.tim");
    service(&mut dos, &mut m, 0x3c00);
    let handle = m.regs.ax;
    for (i, byte) in b"0123456789".iter().enumerate() {
        m.memory.write8(0x2000, 0x200 + i as u16, *byte);
    }
    (m.regs.bx, m.regs.cx, m.regs.dx) = (handle, 10, 0x200);
    service(&mut dos, &mut m, 0x4000);
    (m.regs.bx, m.regs.cx, m.regs.dx) = (handle, 0, 4);
    service(&mut dos, &mut m, 0x4200);
    assert_eq!((m.regs.dx, m.regs.ax), (0, 4), "the new position");
    assert_eq!(read(&mut dos, &mut m, handle, 3), b"456");
    assert!(folders.root.join("saves/ldmg/ldm.tim").exists(), "created under the saves folder");
    assert!(!folders.root.join("game/ldmg/ldm.tim").exists());
}

#[test]
fn parent_paths_are_refused() {
    let folders = Folders::new("parent");
    let (mut dos, mut m) = (folders.dos(), Machine::new());
    name(&mut m, "..\\outside.txt");
    m.regs.ax = 0x3c00;
    assert!(dos.interrupt(&mut m, 0x21).is_err());
}
