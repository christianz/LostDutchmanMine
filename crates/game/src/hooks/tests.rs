//! Every hook on a synthetic machine: the data segment in place, no game code.

use std::path::PathBuf;

use machine::{Flag, LOAD_SEGMENT, Machine};
use patches::resume::{AFTER_HAZARD_CHECK, SELECTION, SELECTOR_CALL, VGA_CHOSEN, WALKING};
use patches::{After, Hook};

use crate::Game;
use crate::symbols::{
    AIM_FLOOR, DATA_SEGMENT, ENCOUNTER_WON, GUNS, ITEM_NONE, ITEM_PAN, LOADED_SCENE, MOUSE_MODE,
    MULES_OWNED, PENDING_DIRECTION, POSITION_X, POSITION_Y, RETURN_X, VGA_CHOICE, inventory,
    joystick, set_joystick,
};

/// The offset after every patched instruction in these tests.
const NEXT: u16 = 42;
/// Space: scan 39h, character 20h.
const SPACE: u32 = 0x39 << 8 | 0x20;
const UP: u16 = 0x48;

fn setup(qol: bool) -> (Machine, Game) {
    let mut m = Machine::new();
    m.regs.ds = DATA_SEGMENT + LOAD_SEGMENT;
    // A stack well clear of the data segment.
    (m.regs.ss, m.regs.sp, m.regs.bp) = (36_864, 4096, 4352);
    let mut g = Game::new(PathBuf::from("data"), PathBuf::from("saves"), qol);
    for slot in 1..11 {
        for row in 0..4 {
            inventory(slot, row).set(&mut m, ITEM_NONE);
        }
    }
    g.dos.mouse.input.move_to(100, 50);
    (m, g)
}

fn run(hook: Hook, m: &mut Machine, g: &mut Game) -> After {
    super::run(hook, m, g, NEXT).expect("the hook runs")
}

fn click(g: &mut Game, x: i32, y: i32) {
    g.dos.mouse.input.move_to(x, y);
    g.dos.mouse.input.buttons(1);
    g.dos.mouse.input.poll();
}

// The walking pointer.

#[test]
fn a_fresh_left_press_is_latched_for_the_selector() {
    let (mut m, mut g) = setup(true);
    click(&mut g, 120, 70);
    m.regs.ax = 0x80;
    assert_eq!(run(Hook::FilterWorldMouse, &mut m, &mut g), After::Continue);
    assert_eq!(m.regs.ax, 0, "walking continues instead of opening the hand");
    assert_eq!(run(Hook::DispatchPendingClick, &mut m, &mut g), After::Goto(SELECTOR_CALL));
}

#[test]
fn a_held_press_is_not_a_second_click() {
    let (mut m, mut g) = setup(true);
    click(&mut g, 120, 70);
    run(Hook::FilterWorldMouse, &mut m, &mut g);
    run(Hook::DispatchWorldClick, &mut m, &mut g);
    g.dos.mouse.input.poll();
    run(Hook::FilterWorldMouse, &mut m, &mut g);
    assert_eq!(run(Hook::DispatchPendingClick, &mut m, &mut g), After::Continue);
}

#[test]
fn classic_walking_keeps_the_original_mouse_read() {
    let (mut m, mut g) = setup(false);
    click(&mut g, 120, 70);
    m.regs.ax = 0x80;
    run(Hook::FilterWorldMouse, &mut m, &mut g);
    assert_eq!(m.regs.ax, 0x80);
    assert_eq!(run(Hook::DispatchPendingClick, &mut m, &mut g), After::Continue);
}

#[test]
fn a_latched_click_fills_the_selectors_locals() {
    let (mut m, mut g) = setup(true);
    click(&mut g, 37, 99);
    run(Hook::FilterWorldMouse, &mut m, &mut g);
    assert_eq!(run(Hook::DispatchWorldClick, &mut m, &mut g), After::Goto(SELECTION));
    let local = |m: &Machine, below: u16| m.memory.read16(m.regs.ss, m.regs.bp - below);
    assert_eq!((local(&m, 2), local(&m, 4), local(&m, 6)), (37, 99, 1));
    assert_eq!(run(Hook::DispatchWorldClick, &mut m, &mut g), After::Continue, "used once");
}

#[test]
fn no_selection_keeps_walking_only_outside_fights() {
    let (mut m, mut g) = setup(true);
    m.regs.ax = 0;
    assert_eq!(run(Hook::KeepWalking, &mut m, &mut g), After::Goto(WALKING));
    m.regs.ax = 3;
    assert_eq!(run(Hook::KeepWalking, &mut m, &mut g), After::Continue);
    m.regs.ax = 0;
    run(Hook::BeginCombat, &mut m, &mut g);
    assert_eq!(run(Hook::KeepWalking, &mut m, &mut g), After::Continue);
}

#[test]
fn selector_result_nine_keeps_the_scene_with_qol() {
    let (mut m, mut g) = setup(true);
    m.regs.ax = 9;
    run(Hook::KeepScene, &mut m, &mut g);
    assert_eq!(m.regs.ax, 0);
    let (mut m, mut g) = setup(false);
    m.regs.ax = 9;
    run(Hook::KeepScene, &mut m, &mut g);
    assert_eq!(m.regs.ax, 9);
}

#[test]
fn the_selector_setup_forgets_a_pending_click() {
    let (mut m, mut g) = setup(true);
    click(&mut g, 37, 99);
    run(Hook::FilterWorldMouse, &mut m, &mut g);
    run(Hook::ResetWorldPointer, &mut m, &mut g);
    assert_eq!(run(Hook::DispatchPendingClick, &mut m, &mut g), After::Continue);
}

#[test]
fn one_mouse_read_sees_a_short_click() {
    let (mut m, mut g) = setup(true);
    g.dos.mouse.input.buttons(1);
    g.dos.mouse.input.buttons(0);
    run(Hook::MousePoll, &mut m, &mut g);
    assert_eq!(g.dos.mouse.input.sample().buttons, 1);
}

// Walking and keys.

#[test]
fn held_directions_walk_twice_as_many_steps_with_qol() {
    let (mut m, mut g) = setup(true);
    set_joystick(&mut m, 4);
    let mut skips = Vec::new();
    for _ in 0..4 {
        run(Hook::BeginWalkTick, &mut m, &mut g);
        skips.push(run(Hook::SkipExtraWalkTick, &mut m, &mut g));
    }
    let skip = After::Goto(NEXT);
    assert_eq!(skips, [skip, After::Continue, skip, After::Continue]);
    assert_eq!(run(Hook::SkipExtraHazardCheck, &mut m, &mut g), After::Continue);
    run(Hook::BeginWalkTick, &mut m, &mut g);
    assert_eq!(run(Hook::SkipExtraHazardCheck, &mut m, &mut g), After::Goto(AFTER_HAZARD_CHECK));
}

#[test]
fn fast_walking_halves_the_delay_argument() {
    let (mut m, mut g) = setup(true);
    set_joystick(&mut m, 1);
    run(Hook::BeginWalkTick, &mut m, &mut g);
    m.push(9);
    run(Hook::HalveWalkDelay, &mut m, &mut g);
    assert_eq!(m.pop(), 5);
    set_joystick(&mut m, 0);
    run(Hook::BeginWalkTick, &mut m, &mut g);
    m.push(9);
    run(Hook::HalveWalkDelay, &mut m, &mut g);
    assert_eq!(m.pop(), 9);
}

#[test]
fn classic_walking_is_never_fast() {
    let (mut m, mut g) = setup(false);
    set_joystick(&mut m, 4);
    run(Hook::BeginWalkTick, &mut m, &mut g);
    assert_eq!(run(Hook::SkipExtraWalkTick, &mut m, &mut g), After::Continue);
}

#[test]
fn only_movement_readers_see_movement_scans() {
    let (mut m, mut g) = setup(true);
    run(Hook::MovementKeys(true), &mut m, &mut g);
    assert!(g.dos.keyboard.movement_aliases);
    run(Hook::MovementKeys(false), &mut m, &mut g);
    assert!(!g.dos.keyboard.movement_aliases);
}

#[test]
fn a_single_key_never_replaces_held_directions() {
    let (mut m, mut g) = setup(true);
    (m.regs.di, m.regs.si) = (UP, 5);
    run(Hook::KeepHeldDirections, &mut m, &mut g);
    assert_eq!(m.regs.di, 0);
    (m.regs.di, m.regs.si) = (UP, 0);
    run(Hook::KeepHeldDirections, &mut m, &mut g);
    assert_eq!(m.regs.di, UP, "a tap without held input keeps its path");
}

#[test]
fn a_direction_in_the_hand_returns_to_walking() {
    let (mut m, mut g) = setup(true);
    m.regs.ax = UP;
    assert_eq!(run(Hook::ReturnToWalking, &mut m, &mut g), After::Goto(WALKING));
    m.regs.ax = 0x1c;
    assert_eq!(run(Hook::ReturnToWalking, &mut m, &mut g), After::Continue);
}

// Scenes.

#[test]
fn the_desert_close_up_waits_for_its_own_dismissal() {
    let (mut m, mut g) = setup(true);
    assert_eq!(run(Hook::DesertCloseUp, &mut m, &mut g), After::Yield);
    assert!(g.dos.waiting && g.desert_view);
    g.dos.keyboard.push(SPACE | dos::KEY_REPEAT);
    assert_eq!(
        run(Hook::DesertCloseUp, &mut m, &mut g),
        After::Yield,
        "a repeat is not a dismissal"
    );
    g.dos.keyboard.push(SPACE);
    PENDING_DIRECTION.set(&mut m, 2);
    assert_eq!(run(Hook::DesertCloseUp, &mut m, &mut g), After::Goto(NEXT));
    assert!(!g.dos.waiting && !g.desert_view && g.dos.keyboard.is_empty());
    assert_eq!(PENDING_DIRECTION.get(&m), 0, "the dismissal never reaches the map");
}

#[test]
fn the_classic_desert_close_up_is_a_timed_preview() {
    let (mut m, mut g) = setup(false);
    assert_eq!(run(Hook::DesertCloseUp, &mut m, &mut g), After::Original);
}

#[test]
fn a_loaded_saloon_keeps_its_saved_doorway() {
    let (mut m, mut g) = setup(true);
    m.regs.ax = 80;
    run(Hook::SaloonReturnPosition, &mut m, &mut g);
    assert_eq!(RETURN_X.get(&m), 80);
    LOADED_SCENE.set(&mut m, 1);
    m.regs.ax = 12;
    run(Hook::SaloonReturnPosition, &mut m, &mut g);
    assert_eq!(RETURN_X.get(&m), 80);
}

#[test]
fn startup_chooses_vga_without_the_selector() {
    let (mut m, mut g) = setup(true);
    assert_eq!(run(Hook::SkipGraphicsSelector, &mut m, &mut g), After::Goto(VGA_CHOSEN));
    assert_eq!(m.regs.ax, VGA_CHOICE);
}

// Supplies.

#[test]
fn panning_needs_a_pan_the_player_or_an_owned_mule_carries() {
    let (mut m, mut g) = setup(true);
    run(Hook::PanOwnership, &mut m, &mut g);
    assert!(m.flag(Flag::Zero), "no pan compares equal to none");
    inventory(4, 2).set(&mut m, ITEM_PAN);
    run(Hook::PanOwnership, &mut m, &mut g);
    assert!(m.flag(Flag::Zero), "an unowned mule carries nothing");
    MULES_OWNED.nth(1).set(&mut m, 1);
    run(Hook::PanOwnership, &mut m, &mut g);
    assert!(!m.flag(Flag::Zero));
}

#[test]
fn panning_starts_only_with_room_and_a_pan() {
    let (mut m, mut g) = setup(true);
    inventory(2, 0).set(&mut m, ITEM_PAN);
    g.dos.keyboard.push(SPACE);
    set_joystick(&mut m, 8);
    run(Hook::BeginPanning, &mut m, &mut g);
    assert!(g.supplies.panning);
    assert!(g.dos.keyboard.is_empty() && joystick(&m) == 0);
    run(Hook::FinishPanning, &mut m, &mut g);
    assert!(!g.supplies.panning);

    for slot in 1..11 {
        inventory(slot, 0).set(&mut m, ITEM_PAN);
    }
    run(Hook::BeginPanning, &mut m, &mut g);
    assert!(!g.supplies.panning, "a full pack goes straight to the original message");
}

#[test]
fn mules_are_sold_cheapest_first_with_qol() {
    let (mut m, mut g) = setup(true);
    run(Hook::MuleForSale(1), &mut m, &mut g);
    assert!(!m.flag(Flag::Zero), "mule 2 waits until mule 1 is sold");
    run(Hook::MuleForSale(0), &mut m, &mut g);
    assert!(m.flag(Flag::Zero));
    let (mut m, mut g) = setup(false);
    run(Hook::MuleForSale(1), &mut m, &mut g);
    assert!(m.flag(Flag::Zero));
}

#[test]
fn held_space_keeps_the_pick_swinging() {
    let (mut m, mut g) = setup(true);
    g.supplies.mining_space_held = true;
    MOUSE_MODE.set(&mut m, 1);
    m.regs.ax = 0;
    run(Hook::ContinueMining, &mut m, &mut g);
    assert_eq!(m.regs.ax, 0x80);
    MOUSE_MODE.set(&mut m, 0);
    m.regs.ax = 0;
    run(Hook::ContinueMining, &mut m, &mut g);
    assert_eq!(m.regs.ax, 0);
}

// Combat.

fn armed(qol: bool) -> (Machine, Game) {
    let (mut m, mut g) = setup(qol);
    GUNS.set(&mut m, 1);
    ENCOUNTER_WON.set(&mut m, 0);
    run(Hook::BeginCombat, &mut m, &mut g);
    (m, g)
}

#[test]
fn an_armed_fight_starts_aiming_with_qol() {
    let (m, g) = armed(true);
    assert_eq!(MOUSE_MODE.get(&m), 1);
    assert!(g.combat.active);
    let (m, _) = {
        let (mut m, mut g) = setup(false);
        GUNS.set(&mut m, 1);
        run(Hook::BeginCombat, &mut m, &mut g);
        (m, g)
    };
    assert_eq!(MOUSE_MODE.get(&m), 0, "classic fights keep their entry mode");
}

#[test]
fn a_left_press_in_the_shooting_area_fires_at_its_position() {
    let (mut m, mut g) = armed(true);
    run(Hook::BeginCombatInput, &mut m, &mut g);
    click(&mut g, 100, 60);
    m.regs.ax = 0x80;
    run(Hook::FilterCombatMouse, &mut m, &mut g);
    assert_eq!(m.regs.ax, 0, "the press does not open the hand");
    run(Hook::FinishCombatInput, &mut m, &mut g);
    assert_eq!(m.regs.ax, 0x80, "the original shot");
    assert_eq!((POSITION_X.get(&m), POSITION_Y.get(&m)), (92, 52), "centred on the pointer");
}

#[test]
fn the_sight_stays_within_the_aiming_area() {
    let (mut m, mut g) = armed(true);
    AIM_FLOOR.set(&mut m, 10);
    run(Hook::BeginCombatInput, &mut m, &mut g);
    g.dos.mouse.input.move_to(319, 111);
    g.dos.mouse.input.poll();
    m.regs.ax = 0;
    run(Hook::FinishCombatInput, &mut m, &mut g);
    assert_eq!((POSITION_X.get(&m), POSITION_Y.get(&m)), (290, 84));
}

#[test]
fn a_won_fight_keeps_its_victory_choices() {
    let (mut m, mut g) = armed(true);
    ENCOUNTER_WON.set(&mut m, 1);
    run(Hook::BeginCombatInput, &mut m, &mut g);
    click(&mut g, 80, 40);
    m.regs.ax = 0x80;
    run(Hook::FilterCombatMouse, &mut m, &mut g);
    assert_eq!(m.regs.ax, 0x80);
}

#[test]
fn the_sight_is_drawn_at_display_cadence_with_qol() {
    let (mut m, mut g) = armed(true);
    assert_eq!(run(Hook::DeferCombatSight, &mut m, &mut g), After::Goto(NEXT));
    assert!(!g.combat.sight_visible);
    run(Hook::ShowCombatSight, &mut m, &mut g);
    assert!(g.combat.sight_visible);
    run(Hook::EndCombat, &mut m, &mut g);
    assert!(!g.combat.active);
    let (mut m, mut g) = armed(false);
    assert_eq!(run(Hook::DeferCombatSight, &mut m, &mut g), After::Continue);
}

// The overlay's view of menus and scenes.

#[test]
fn context_buttons_come_from_the_scenes_arguments() {
    let (mut m, mut g) = setup(true);
    for (i, on) in [1u8, 0, 1, 1].into_iter().enumerate() {
        let flag = 0x40 + i as u16;
        m.memory.write16(m.regs.ss, m.regs.sp + 4 + 2 * i as u16, flag);
        m.memory.write8(m.regs.ds, flag, on);
    }
    run(Hook::ReadContextButtons, &mut m, &mut g);
    assert_eq!(g.overlay.context_buttons, 0b1101);
    run(Hook::OpenBuildingMenu, &mut m, &mut g);
    assert_eq!(g.overlay.context_buttons, 0, "a menu covers the scene's buttons");
    run(Hook::CloseBuildingMenu, &mut m, &mut g);
    assert_eq!(g.overlay.context_buttons, 0b1101);
    run(Hook::ClearContextButtons, &mut m, &mut g);
    assert_eq!(g.overlay.context_buttons, 0);
}

#[test]
fn toolbar_clicks_reach_the_original_panel_buttons() {
    let (mut m, mut g) = setup(true);
    g.dos.video.mode = 0x13;
    m.memory.write16(m.regs.ss, m.regs.sp + 4, 52 + 2 * 42 + 5);
    m.memory.write16(m.regs.ss, m.regs.sp + 6, 170);
    run(Hook::OpenSelectorMenu, &mut m, &mut g);
    let argument = |m: &Machine, at: u16| m.memory.read16(m.regs.ss, m.regs.sp + at);
    assert_eq!((argument(&m, 4), argument(&m, 6)), (69 + 2 * 42, 181));
    run(Hook::CloseSelectorMenu, &mut m, &mut g);
}

#[test]
fn the_mule_shop_marks_sold_mules_only_while_its_offers_show() {
    let (mut m, mut g) = setup(true);
    run(Hook::ShowMuleShop, &mut m, &mut g);
    assert!(!g.overlay.mule_shop_visible, "not in the shop");
    run(Hook::EnterMuleShop, &mut m, &mut g);
    run(Hook::ShowMuleShop, &mut m, &mut g);
    assert!(g.overlay.mule_shop_visible);
    run(Hook::HideMuleShop, &mut m, &mut g);
    assert!(!g.overlay.mule_shop_visible);
    run(Hook::ShowMuleShop, &mut m, &mut g);
    run(Hook::LeaveMuleShop, &mut m, &mut g);
    assert!(!g.overlay.mule_shop && !g.overlay.mule_shop_visible);
}

#[test]
fn the_saloon_sleep_is_a_menu_without_hover() {
    let (mut m, mut g) = setup(true);
    g.overlay.context_buttons = 3;
    run(Hook::BeginSleep, &mut m, &mut g);
    assert!(g.overlay.menu_open());
    run(Hook::EndSleep, &mut m, &mut g);
    assert!(!g.overlay.menu_open());
    assert_eq!(g.overlay.context_buttons, 3);
}
