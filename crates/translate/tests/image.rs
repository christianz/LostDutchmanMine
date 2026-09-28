//! Unpacking EXEPACK executables, on a hand-built file and on the real game.

use machine::Address;
use translate::image::{ImageError, load, unpack};

/// An MZ file whose EXEPACK data unpacks to `ABCD` `wxyz` and eight 0x90 bytes,
/// with one relocation at offset 3.
fn packed() -> Vec<u8> {
    let mut module = Vec::new();
    module.extend(b"ABCD"); // Stored verbatim: the unpacked prefix.
    module.extend(b"wxyz"); // Literal run...
    module.extend(4_u16.to_le_bytes());
    module.push(0xb3); // ...copy 4 bytes; the last command.
    module.push(0x90); // Fill byte...
    module.extend(8_u16.to_le_bytes());
    module.push(0xb0); // ...fill 8 bytes.
    while module.len() % 16 != 0 {
        module.push(0xff); // Padding the unpacker skips.
    }
    let stub_paragraph = (module.len() / 16) as u16;

    let marker = b"Packed file is corrupt";
    let code = [0xcc; 6];
    let table = 2 * 16 + 2;
    let size = (18 + code.len() + marker.len() + table) as u16;
    for word in [0x0123, 0x0045, 0, size, 0x0800, 0x0067, 1, 1, 0x4252] {
        module.extend(u16::to_le_bytes(word));
    }
    module.extend(code);
    module.extend(marker);
    module.extend(1_u16.to_le_bytes());
    module.extend(3_u16.to_le_bytes());
    for _ in 1..16 {
        module.extend(0_u16.to_le_bytes());
    }

    let mut header = [0_u16; 16];
    header[0] = u16::from_le_bytes(*b"MZ");
    header[4] = 2; // Header paragraphs.
    header[11] = stub_paragraph; // Initial CS: the EXEPACK stub.
    let mut file: Vec<u8> = header.iter().flat_map(|w| w.to_le_bytes()).collect();
    file.extend(module);
    file
}

#[test]
fn unpacks_fill_and_literal_runs_backwards() {
    let image = unpack(&packed()).expect("valid EXEPACK file");
    assert_eq!(image.bytes, b"ABCDwxyz\x90\x90\x90\x90\x90\x90\x90\x90");
    assert_eq!(image.relocations, [3]);
    assert_eq!(image.entry, Address::new(0x0045, 0x0123));
    assert_eq!((image.stack_segment, image.stack_pointer), (0x0067, 0x0800));
}

#[test]
fn rejects_files_that_are_not_the_supported_release() {
    let error = load(&packed()).expect_err("a synthetic file is not LDM.EXE");
    assert!(matches!(error, ImageError::UnsupportedRelease { .. }), "{error}");
}

#[test]
fn rejects_truncated_and_foreign_files() {
    assert!(matches!(unpack(b"ZM"), Err(ImageError::NotMz)));
    let mut file = packed();
    file.truncate(file.len() - 20);
    assert!(unpack(&file).is_err(), "truncated relocation table");
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn unpacks_the_original_executable() {
    let path = std::env::var("LDM_EXE").expect("LDM_EXE");
    let image =
        load(&std::fs::read(path).expect("readable LDM.EXE")).expect("the supported release");
    assert_eq!(image.bytes.len(), 484_448);
    assert_eq!(image.relocations.len(), 2_413);
    assert_eq!(image.entry, Address::new(0x13b4, 0x00ca));
    assert_eq!((image.stack_segment, image.stack_pointer), (0x795c, 0x0800));
    assert_eq!(
        translate::image::sha256_hex(&image.bytes),
        "cccd45eee828efaed64f2d3306ceaefa90351f1864ebc6fdec238ce707e58631"
    );
}
