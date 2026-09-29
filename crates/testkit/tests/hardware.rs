//! Every opcode form the game uses, against SingleStepTests 8088 hardware captures.
//!
//! Run `cargo xtask vectors` once to download the vectors into `.local/vectors`.
//! `LDM_VECTORS_LIMIT` bounds the vectors per file (default 1000; `all` for 10,000).

use std::path::PathBuf;

use testkit::vectors::{Summary, flag_masks, run_file, stems};

#[test]
#[ignore = "needs the downloaded vectors: cargo xtask vectors, then cargo xtask test"]
fn every_opcode_form_matches_the_8088() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.local/vectors");
    let masks = flag_masks(&root.join("metadata.json")).expect("metadata.json");
    let limit = match std::env::var("LDM_VECTORS_LIMIT").as_deref() {
        Ok("all") => usize::MAX,
        Ok(n) => n.parse().expect("a number or `all`"),
        Err(_) => 1000,
    };
    let mut summary = Summary::default();
    for stem in stems() {
        let path = root.join(format!("{stem}.json.gz"));
        run_file(&path, masks.get(stem).copied().unwrap_or(0xffff), limit, &mut summary)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    }
    println!("{} vectors match the hardware", summary.passed);
    for (reason, count) in &summary.skipped {
        println!("skipped {count}: {reason}");
    }
    for (mnemonic, count) in &summary.unlowered {
        println!("  not lowered: {mnemonic} x{count}");
    }
    for (reason, count) in &summary.relaxed {
        println!("relaxed {count}: {reason}");
    }
    for (mnemonic, count) in summary.failures_by_mnemonic() {
        println!("FAIL {mnemonic} x{count}");
        let examples =
            summary.failures.iter().filter(|f| f.split([' ', ':']).next() == Some(mnemonic));
        for failure in examples.take(3) {
            println!("    {failure}");
        }
    }
    assert!(
        summary.failures.is_empty(),
        "{} vectors differ from the hardware",
        summary.failures.len()
    );
}
