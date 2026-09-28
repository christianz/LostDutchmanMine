//! The Rust written for a small hand-built program, reviewed as a snapshot.

use machine::Address;
use patches::{Action, Hook, Patch};
use translate::emit::emit;
use translate::image::LoadImage;
use translate::recover::recover;
use translate::translation::Translation;

fn image() -> LoadImage {
    let mut bytes = vec![0xcc; 0x21];
    let code: &[u8] = &[
        0xb8, 0x34, 0x12, // 0000 mov ax, 0x1234
        0x89, 0x07, // 0003 mov [bx], ax
        0x83, 0xc0, 0x01, // 0005 add ax, 1
        0x75, 0x04, // 0008 jne 000e
        0xe8, 0x06, 0x00, // 000a call 0013
        0x90, // 000d nop (return point)
        0xcd, 0x21, // 000e int 21h
        0xeb, 0xee, // 0010 jmp 0000 (backward)
    ];
    bytes[..code.len()].copy_from_slice(code);
    let routine: &[u8] = &[
        0x55, // 0013 push bp (Before)
        0x83, 0x3e, 0xdc, 0x53, 0x00, // 0014 cmp word [53dc], 0 (Replace)
        0x9a, 0x00, 0x00, 0x01, 0x00, // 0019 lcall 0001:0000
        0xc3, // 001e ret
    ];
    bytes[0x13..0x13 + routine.len()].copy_from_slice(routine);
    bytes[0x20] = 0xcb; // 0001:0000 retf
    LoadImage {
        bytes,
        relocations: vec![0x1c],
        entry: Address::new(0, 0),
        stack_segment: 0,
        stack_pointer: 0x100,
    }
}

const PATCHES: &[Patch] = &[
    Patch {
        name: "sample before",
        site: Address::new(0, 0x13),
        expect: &[0x55],
        action: Action::Before(Hook::MousePoll),
    },
    Patch {
        name: "sample replace",
        site: Address::new(0, 0x14),
        expect: &[0x83, 0x3e, 0xdc, 0x53, 0x00],
        action: Action::Replace(Hook::PanOwnership),
    },
];

fn files() -> Vec<translate::emit::File> {
    let image = image();
    let translation =
        Translation::build(&image, &recover(&image, &[]), PATCHES).expect("translation");
    emit(&translation, "sample")
}

#[test]
fn dispatcher_covers_every_segment() {
    insta::assert_snapshot!(files()[0].contents);
}

#[test]
fn segment_blocks_follow_the_original_code() {
    let files = files();
    assert_eq!(files[1].name, "segment_0000.rs");
    insta::assert_snapshot!(files[1].contents);
}

#[test]
fn a_patch_on_the_wrong_bytes_stops_the_build() {
    let image = image();
    let wrong = [Patch { expect: &[0x56], ..PATCHES[0] }];
    let error = Translation::build(&image, &recover(&image, &[]), &wrong).expect_err("mismatch");
    assert_eq!(error.to_string(), "patch `sample before` at 0000:0013: expected 56, found 55");
}
