//! What each hook does. The patch table in `patches` says where hooks run;
//! this table says what they are, one function per hook, grouped by concern.

mod combat;
mod pointer;
mod scenes;
mod supplies;
mod walking;

#[cfg(test)]
mod tests;

use engine::Stop;
use machine::Machine;
use patches::{After, Hook};

pub(crate) use combat::{Combat, aim};
pub(crate) use pointer::WorldPointer;
pub(crate) use supplies::{Supplies, mule_available};
pub(crate) use walking::Walk;

use crate::{Game, overlay};

/// Runs `hook`; `next` is the offset after its instruction.
///
/// # Errors
///
/// [`Stop`] when a hook cannot do its work, such as reading the panel artwork.
pub(crate) fn run(hook: Hook, m: &mut Machine, g: &mut Game, next: u16) -> Result<After, Stop> {
    Ok(match hook {
        Hook::DesertCloseUp => scenes::desert_close_up(m, g, next),
        Hook::SaloonReturnPosition => scenes::saloon_return_position(m),
        Hook::SkipGraphicsSelector => scenes::skip_graphics_selector(m),
        Hook::PanningRewardLoop => supplies::panning_reward_loop(m, g),
        Hook::PanningFrameLoop => supplies::panning_frame_loop(m),
        Hook::PanOwnership => supplies::pan_ownership(m),
        Hook::BeginPanning => supplies::begin_panning(m, g),
        Hook::FinishPanning => supplies::finish_panning(m, g),
        Hook::MuleForSale(index) => supplies::mule_for_sale(m, g, index),
        Hook::ContinueMining => supplies::continue_mining(m, g),
        Hook::MousePoll => pointer::poll(g),
        Hook::FilterWorldMouse => pointer::filter(m, g),
        Hook::ResetWorldPointer => pointer::reset(g),
        Hook::DispatchPendingClick => pointer::dispatch_pending(g),
        Hook::KeepScene => pointer::keep_scene(m, g),
        Hook::DispatchWorldClick => pointer::dispatch(m, g),
        Hook::KeepWalking => pointer::keep_walking(m, g),
        Hook::BeginCombat => combat::begin(m, g),
        Hook::BeginCombatInput => combat::begin_input(m, g),
        Hook::FilterCombatMouse => combat::filter_mouse(m, g),
        Hook::FinishCombatInput => combat::finish_input(m, g),
        Hook::ShowCombatSight => combat::show_sight(g),
        Hook::EndCombat => combat::end(g),
        Hook::DeferCombatSight => combat::defer_sight(g, next),
        Hook::BeginWalkTick => walking::begin_tick(m, g),
        Hook::SkipExtraWalkTick => walking::skip_extra_tick(g, next),
        Hook::HalveWalkDelay => walking::halve_delay(m, g),
        Hook::SkipExtraHazardCheck => walking::skip_extra_hazard_check(g),
        Hook::MovementKeys(movement) => walking::movement_keys(g, movement),
        Hook::KeepHeldDirections => walking::keep_held_directions(m),
        Hook::ReturnToWalking => walking::return_to_walking(m),
        Hook::PreparePanel => overlay::prepare_panel(g)?,
        Hook::ReadContextButtons => overlay::read_context_buttons(m, g),
        Hook::ClearContextButtons => overlay::clear_context_buttons(g),
        Hook::OpenSelectorMenu => overlay::open_selector_menu(m, g),
        Hook::CloseSelectorMenu | Hook::CloseBuildingMenu | Hook::EndSleep => {
            overlay::close_menu(g)
        }
        Hook::OpenBuildingMenu => overlay::open_building_menu(g),
        Hook::BeginSleep => overlay::open_menu(g),
        Hook::EnterMuleShop => overlay::enter_mule_shop(g),
        Hook::LeaveMuleShop => overlay::leave_mule_shop(g),
        Hook::ShowMuleShop => overlay::show_mule_shop(g),
        Hook::HideMuleShop => overlay::hide_mule_shop(g),
    })
}

/// Input that belongs to one screen must never reach the next: forget queued
/// keys, clicks and a forwarded direction.
pub(crate) fn forget_input(m: &mut Machine, g: &mut Game) {
    g.dos.keyboard.clear();
    crate::symbols::PENDING_DIRECTION.set(m, 0);
    g.dos.mouse.input.clear();
    g.pointer.reset(g.dos.mouse.input.current());
}
