//! The C++ `poker` test: the original C runtime qsort sorts a poker hand through
//! poker's far comparator callback, the callback the first native poker build
//! lacked, for every order of five cards.

use machine::Address;
use testkit::harness::{Harness, STACK_TOP};

/// The C runtime's qsort.
const QSORT: Address = Address::new(0x13b4, 0x227c);
/// Poker's card comparator, which qsort calls back far.
const COMPARE_CARDS: Address = Address::new(0x0106, 0x0000);
/// Where the hand is laid out in the data segment.
const HAND: u16 = 0x6000;
/// Each card's record: its rank, then its number.
const RECORD_SIZE: u16 = 4;
/// Each card's rank, by card number.
const RANKS: [u16; 5] = [2, 7, 14, 10, 3];
/// qsort sorts five cards well within this many steps.
const SORT_LIMIT: u64 = 100_000;
/// The caller releases qsort's arguments: base, count, size and a far pointer.
const ARGUMENT_BYTES: u16 = 10;

/// Rearranges `order` into the next permutation in lexicographic order, as
/// C++'s `std::next_permutation` does; false once it wraps to the first.
fn next_permutation(order: &mut [usize]) -> bool {
    let Some(pivot) = order.windows(2).rposition(|pair| pair[0] < pair[1]) else {
        order.reverse();
        return false;
    };
    let successor = order.iter().rposition(|&card| card > order[pivot]).expect("a larger card");
    order.swap(pivot, successor);
    order[pivot + 1..].reverse();
    true
}

/// Sorts the hand dealt in `order` and checks it comes back highest first.
fn sort(order: &[usize]) {
    let mut h = Harness::loaded();
    let ds = h.m.regs.ds;
    for (i, &card) in (0..).zip(order) {
        h.m.memory.write16(ds, HAND + RECORD_SIZE * i, RANKS[card]);
        h.m.memory.write16(ds, HAND + RECORD_SIZE * i + 2, card as u16);
    }
    let count = RANKS.len() as u16;
    let comparator = (COMPARE_CARDS.offset, COMPARE_CARDS.runtime_segment());
    h.call(QSORT, &[HAND, count, RECORD_SIZE, comparator.0, comparator.1]);
    h.until(SORT_LIMIT, "Poker sort did not return", Harness::returned);
    assert_eq!(h.m.regs.sp, STACK_TOP - ARGUMENT_BYTES, "Poker callback corrupted stack");
    let mut previous = u16::MAX;
    let mut seen = [false; 5];
    for i in 0..count {
        let rank = h.m.memory.read16(ds, HAND + RECORD_SIZE * i);
        let card = usize::from(h.m.memory.read16(ds, HAND + RECORD_SIZE * i + 2));
        assert!(
            rank <= previous && card < seen.len() && rank == RANKS[card] && !seen[card],
            "Poker cards sorted incorrectly"
        );
        previous = rank;
        seen[card] = true;
    }
}

#[test]
fn next_permutation_visits_every_order_once() {
    let mut order = [0, 1, 2];
    let mut orders = vec![order];
    while next_permutation(&mut order) {
        orders.push(order);
    }
    assert_eq!(orders, [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]]);
    assert_eq!(order, [0, 1, 2], "it wraps to the first order");
}

#[test]
#[ignore = "needs the original game: LDM_EXE=/path/to/LDM.EXE cargo xtask verify"]
fn every_hand_sorts_through_the_original_qsort_and_callback() {
    let mut order = [0, 1, 2, 3, 4];
    let mut hands = 0;
    loop {
        sort(&order);
        hands += 1;
        if !next_permutation(&mut order) {
            break;
        }
    }
    assert_eq!(hands, 120, "all poker-hand permutations");
}
