//! `tests/verify-panning.py`: the river's original panning animation plays with
//! QoL on and off and pays one deferred bag, input during it is drained, F11
//! pauses it, walking works afterward, and both bags survive the original
//! save format.

use game::Report;

use super::fields::{BOUNDARIES, BUILDING, GOLD_BAGS, PANNING_ACTIVE, TOWN_PAGE, X, Y};
use super::run::Run;

/// The slot the script saves to, and the original format's size of it.
const SLOT: &str = "LDMSAVE8.SAV";
const SAVE_SIZE: usize = 6066;
/// Where the gold count sits in a save: in the fourth original save block,
/// which holds DS:53d4 through 5405.
const GOLD_OFFSET: usize = 0x36 + 0x20 + 0x24 + 0x53ea - 0x53d4;

/// Checks the `panning` run.
///
/// # Errors
///
/// The first expectation that fails.
pub fn check(run: &Run) -> Result<(), String> {
    let ids = [700, 701, 702, 703, 706, 707, 708, 709, 710, 711, 712, 713, 714, 715];
    let [s700, s701, s702, s703, s706, s707, s708, s709, s710, s711, s712, s713, s714, s715] =
        run.reports(ids)?;
    state(700, s700, false, 0)?;
    state(701, s701, true, 0)?;
    state(702, s702, true, 0)?;
    state(706, s706, false, 1)?;
    ensure!(s706.x == s700.x && s706.y == s700.y, "capture 706 left the river: {s706:?}");
    ensure!(s707.x > s706.x && s707.gold_bags == 1, "capture 707 did not walk on: {s707:?}");
    ensure!(!s715.qol && s715.gold_bags == 1, "capture 715: {s715:?}");
    // Since update 16 the original animation also plays with QoL off. The Pan
    // click made after re-enabling QoL lands during that animation and is ignored.
    ensure!(!s708.qol, "capture 708 must have QoL off");
    state(708, s708, true, 1)?;
    ensure!(s709.qol, "capture 709 must have QoL on");
    state(709, s709, true, 1)?;
    state(703, s703, true, 1)?;
    state(710, s710, false, 2)?;
    for field in [X, Y, TOWN_PAGE, BUILDING, GOLD_BAGS] {
        ensure!(field.of(s711) == field.of(s712), "{}: {s711:?} {s712:?}", field.key);
    }
    for field in [BOUNDARIES, PANNING_ACTIVE, X, Y, GOLD_BAGS] {
        ensure!(field.of(s713) == field.of(s714), "{}: {s713:?} {s714:?}", field.key);
    }
    let data = run.save(SLOT)?;
    ensure!(data.len() == SAVE_SIZE, "{SLOT}: {} bytes, expected {SAVE_SIZE}", data.len());
    let gold = data
        .get(GOLD_OFFSET..GOLD_OFFSET + 2)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u16::from_le_bytes);
    ensure!(gold == Some(2), "{SLOT} holds {gold:?} gold bags, expected 2");
    ensure!(
        run.save("LDMSAVE.LDM")?.windows(7).any(|name| name == b"PANGOLD"),
        "LDMSAVE.LDM does not name the save PANGOLD"
    );
    Ok(())
}

/// Whether capture `n` shows `panning` with `bags` bags of gold.
fn state(n: u32, s: &Report, panning: bool, bags: u16) -> Result<(), String> {
    ensure!(
        s.panning == panning && s.gold_bags == bags,
        "capture {n}: panning {} with {} bags, expected panning {panning} with {bags}",
        s.panning,
        s.gold_bags
    );
    Ok(())
}
