//! Control-flow recovery on a hand-built image and on the real game.

use std::collections::BTreeSet;

use machine::Address;
use translate::image::LoadImage;
use translate::recover::{parse_entry_points, recover};

fn image() -> LoadImage {
    let mut bytes = vec![0; 0x20];
    bytes[..0x0a].copy_from_slice(&[
        0xe8, 0x05, 0x00, // 0000:0000 call 0008
        0x9a, 0x00, 0x00, 0x01, 0x00, // 0000:0003 lcall 0001:0000
        0xeb, 0xfe, // 0000:0008 jmp 0008
    ]);
    bytes[0x0a] = 0xc3; // Data: never reached.
    bytes[0x10..0x15].copy_from_slice(&[
        0xff, 0xd0, // 0001:0000 call ax
        0xcd, 0x21, // 0001:0002 int 21h
        0xc3, // 0001:0004 ret
    ]);
    LoadImage {
        bytes,
        relocations: vec![6],
        entry: Address::new(0, 0),
        stack_segment: 0,
        stack_pointer: 0x100,
    }
}

#[test]
fn follows_direct_transfers_and_stops_at_terminators() {
    let recovered = recover(&image(), &[]);
    let found: Vec<Address> = recovered.instructions.keys().copied().collect();
    let expected =
        [(0, 0), (0, 3), (0, 8), (1, 0), (1, 2), (1, 4)].map(|(s, o)| Address::new(s, o));
    assert_eq!(found, expected);
    assert!(recovered.targets.contains(&Address::new(0, 8)), "call target");
    assert!(recovered.targets.contains(&Address::new(1, 0)), "far call target");
    assert_eq!(recovered.indirect, [Address::new(1, 0)], "indirect call is recorded, not followed");
}

#[test]
fn extra_seeds_are_recovered_too() {
    let mut image = image();
    image.bytes[0x0b] = 0x90; // nop at 0000:000b
    image.bytes[0x0c] = 0xc3; // ret at 0000:000c
    let recovered = recover(&image, &[Address::new(0, 0x0b)]);
    assert!(recovered.instructions.contains_key(&Address::new(0, 0x0c)));
    assert!(!recovered.instructions.contains_key(&Address::new(0, 0x0a)), "data stays data");
}

#[test]
fn entry_points_parse_with_their_evidence() {
    let points = parse_entry_points(
        r#"
        [[group]]
        why = "A comparator passed to qsort."
        at = ["0106:0000", "1613:1c7d"]
        "#,
    )
    .expect("valid entry points");
    let expected: BTreeSet<Address> =
        [Address::new(0x0106, 0), Address::new(0x1613, 0x1c7d)].into();
    assert_eq!(points.into_iter().collect::<BTreeSet<_>>(), expected);
    assert!(
        parse_entry_points("[[group]]\nat = [\"0106:0000\"]\n").is_err(),
        "a reason is required"
    );
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn recovers_the_original_game_like_the_python_translator() {
    let exe = std::fs::read(std::env::var("LDM_EXE").expect("LDM_EXE")).expect("readable LDM.EXE");
    let image = translate::image::load(&exe).expect("the supported release");
    let seeds =
        parse_entry_points(include_str!("../../game/entry_points.toml")).expect("entry points");
    let recovered = recover(&image, &seeds);
    assert_eq!(recovered.instructions.len(), 35_338, "the Python translator recovered 35,338");

    let relocations = translate::Relocations::from_offsets(image.relocations.iter().copied());
    let unsupported: Vec<String> = recovered
        .instructions
        .keys()
        .filter_map(|&at| {
            let linear = usize::from(at.segment) * 16 + usize::from(at.offset);
            let bytes = &image.bytes[linear..image.bytes.len().min(linear + 15)];
            translate::lower(at, bytes, &relocations).err().map(|error| error.to_string())
        })
        .collect();
    assert!(
        unsupported.is_empty(),
        "{} instructions do not lower: {:?}",
        unsupported.len(),
        &unsupported[..unsupported.len().min(10)]
    );
}
