//! `tests/console.cpp`: the original C runtime's `kbhit` and `getch` poll
//! without waiting or consuming input, keep Escape readable, recognise keys
//! without a character, and still serve the runtime's own buffered character.

use game::symbols::Global;
use machine::Address;
use testkit::harness::{Harness, STACK_TOP};

/// The C runtime's `kbhit`.
const KBHIT: Address = Address::new(0x13b4, 0x1872);
/// The C runtime's `getch`.
const GETCH: Address = Address::new(0x13b4, 0x1898);
/// `getch`'s one buffered character.
const BUFFERED: Global = Global(0x1f4c);
/// What the buffer holds when empty.
const NOTHING_BUFFERED: u16 = 0xffff;
/// An optional input hook the runtime calls first; zero for none.
const INPUT_HOOK: Global = Global(0x20a6);
/// A helper returns well within this many steps.
const HELPER_LIMIT: u64 = 1000;
/// The key `a`.
const A: u32 = 0x1e61;
/// Escape.
const ESCAPE: u32 = 0x011b;
/// Up, a key without a character.
const UP: u32 = 0x4800;

/// Calls the original console helper at `routine` and returns AX.
fn call(h: &mut Harness, routine: Address) -> u16 {
    h.call(routine, &[]);
    let start = h.m.steps;
    while !h.returned() {
        assert!(h.m.steps - start < HELPER_LIMIT, "Original console helper did not return");
        h.step();
        assert!(!h.game.dos.waiting, "Console status must never wait for a key");
    }
    assert_eq!(h.m.regs.sp, STACK_TOP, "Console helper corrupted stack");
    h.m.regs.ax
}

/// The keys waiting, oldest first.
fn queued(h: &mut Harness) -> Vec<u32> {
    h.keyboard().iter().collect()
}

/// Replaces the waiting keys with `keys`.
fn queue(h: &mut Harness, keys: &[u32]) {
    h.keyboard().clear();
    for &key in keys {
        h.keyboard().push(key);
    }
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn kbhit_and_getch_poll_without_waiting_or_consuming_input() {
    let mut h = Harness::loaded();
    h.set(BUFFERED, NOTHING_BUFFERED);
    h.set(INPUT_HOOK, 0);
    assert_eq!(call(&mut h, KBHIT), 0, "Empty keyboard must report no input");

    queue(&mut h, &[A, ESCAPE]);
    let pending = queued(&mut h);
    assert!(
        call(&mut h, KBHIT) == 0xff && call(&mut h, KBHIT) == 0xff,
        "Pending input must stay available across repeated status checks"
    );
    assert_eq!(queued(&mut h), pending, "Status check consumed a key");
    assert!(
        call(&mut h, GETCH) == u16::from(b'a') && queued(&mut h).len() == 1,
        "Original getch lost the queued character"
    );
    assert!(
        call(&mut h, KBHIT) == 0xff && call(&mut h, GETCH) == 0x1b,
        "Escape must remain readable after the status check"
    );
    assert!(
        call(&mut h, KBHIT) == 0 && queued(&mut h).is_empty(),
        "Status did not return to empty after reading input"
    );

    queue(&mut h, &[UP]);
    let pending = queued(&mut h);
    assert!(
        call(&mut h, KBHIT) == 0xff && queued(&mut h) == pending,
        "A non-character key must still report pending input"
    );

    queue(&mut h, &[]);
    h.set(BUFFERED, u16::from(b'z'));
    assert!(
        call(&mut h, KBHIT) == 0xff && call(&mut h, GETCH) == u16::from(b'z'),
        "Original C-runtime buffered input changed"
    );
    assert_eq!(call(&mut h, KBHIT), 0, "C-runtime buffered character did not clear");
}
