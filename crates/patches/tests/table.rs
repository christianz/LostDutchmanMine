//! The patch table's structure, independent of the game's bytes.

use std::collections::BTreeMap;

use patches::{Action, PATCHES};

#[test]
fn each_site_has_one_meaning() {
    let mut sites: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for patch in PATCHES {
        sites.entry(patch.site).or_default().push(patch);
    }
    for (site, patches) in &sites {
        let replaced = patches.iter().filter(|p| matches!(p.action, Action::Replace(_))).count();
        assert!(replaced == 0 || patches.len() == 1, "{site}: a replacement stands alone");
        assert!(
            patches.iter().all(|p| p.expect == patches[0].expect),
            "{site}: one instruction, one expectation"
        );
    }
}

#[test]
fn every_patch_explains_itself() {
    assert!(PATCHES.iter().all(|patch| !patch.name.is_empty() && !patch.expect.is_empty()));
}
