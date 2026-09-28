//! The patch table. Several patches at one site run in the order listed here.

use machine::Address;

use crate::{Action, Hook, Patch, Routine};

const fn at(segment: u16, offset: u16) -> Address {
    Address::new(segment, offset)
}

const fn before(name: &'static str, site: Address, expect: &'static [u8], hook: Hook) -> Patch {
    Patch { name, site, expect, action: Action::Before(hook) }
}

const fn replace(name: &'static str, site: Address, expect: &'static [u8], hook: Hook) -> Patch {
    Patch { name, site, expect, action: Action::Replace(hook) }
}

const fn routine(
    name: &'static str,
    site: Address,
    expect: &'static [u8],
    routine: Routine,
) -> Patch {
    Patch { name, site, expect, action: Action::Routine(routine) }
}

const LCALL_DESERT_DELAY: &[u8] = &[0x9a, 0x0c, 0x00, 0x05, 0x05];
const LCALL_WALK_CLOCK: &[u8] = &[0x9a, 0xe6, 0x00, 0x05, 0x05];
const LCALL_WALK_DELAY: &[u8] = &[0x9a, 0x8c, 0x00, 0x05, 0x05];
const LCALL_SIGHT_BLIT: &[u8] = &[0x9a, 0x8a, 0x0e, 0xc5, 0x0f];
const LCALL_KEY_READER: &[u8] = &[0x9a, 0x3a, 0x00, 0xa7, 0x0f];
const LCALL_COMMAND_READER: &[u8] = &[0x9a, 0x40, 0x00, 0x68, 0x0b];
const PUSH_BP: &[u8] = &[0x55];
const POP_DI: &[u8] = &[0x5f];
const RETF: &[u8] = &[0xcb];

/// Every patch, in application order.
pub static PATCHES: &[Patch] = &[
    // The desert close-up (Space on the map).
    replace(
        "desert close-up waits for dismissal",
        at(0x05d6, 0x0627),
        LCALL_DESERT_DELAY,
        Hook::DesertCloseUp,
    ),
    replace(
        "desert close-up waits for dismissal",
        at(0x05d6, 0x064f),
        LCALL_DESERT_DELAY,
        Hook::DesertCloseUp,
    ),
    replace(
        "desert close-up waits for dismissal",
        at(0x05d6, 0x06cf),
        LCALL_DESERT_DELAY,
        Hook::DesertCloseUp,
    ),
    // Buildings.
    replace(
        "saloon exit uses the loaded doorway",
        at(0x08c0, 0x0088),
        &[0xa3, 0x60, 0x5b],
        Hook::SaloonReturnPosition,
    ),
    // Panning: the restored animation and actual pan ownership.
    replace(
        "restore the panning reward loop",
        at(0x033f, 0x0303),
        &[0xe9, 0x0a, 0x01],
        Hook::PanningRewardLoop,
    ),
    replace(
        "restore the panning frames",
        at(0x033f, 0x039e),
        &[0xeb, 0x03],
        Hook::PanningFrameLoop,
    ),
    replace(
        "Pan label needs a carried pan",
        at(0x033f, 0x00d8),
        &[0x83, 0x3e, 0xdc, 0x53, 0x00],
        Hook::PanOwnership,
    ),
    replace(
        "Pan needs a carried pan",
        at(0x033f, 0x02d5),
        &[0x83, 0x3e, 0xdc, 0x53, 0x00],
        Hook::PanOwnership,
    ),
    before("panning begins", at(0x033f, 0x02cc), PUSH_BP, Hook::BeginPanning),
    before("panning ends", at(0x033f, 0x0432), POP_DI, Hook::FinishPanning),
    // Startup.
    replace(
        "VGA without the graphics selector",
        at(0x1265, 0x0693),
        &[0xba, 0x22, 0xfa],
        Hook::SkipGraphicsSelector,
    ),
    // Mouse input.
    before("one mouse sample per original read", at(0x0fc5, 0x0038), PUSH_BP, Hook::MousePoll),
    before(
        "walking pointer coexists with the keys",
        at(0x0fa7, 0x0011),
        &[0x25, 0x80, 0x00],
        Hook::FilterWorldMouse,
    ),
    before(
        "left press in the shooting area fires",
        at(0x0fa7, 0x0011),
        &[0x25, 0x80, 0x00],
        Hook::FilterCombatMouse,
    ),
    before(
        "keep the physical pointer",
        at(0x0fa7, 0x0032),
        &[0xc7, 0x06, 0x04, 0x5a, 0x01, 0x00],
        Hook::ResetWorldPointer,
    ),
    // Combat.
    before("fight begins", at(0x040a, 0x0002), PUSH_BP, Hook::BeginCombat),
    before(
        "aiming read begins",
        at(0x040a, 0x0288),
        &[0x9a, 0x06, 0x00, 0xa7, 0x0f],
        Hook::BeginCombatInput,
    ),
    before("aiming read ends", at(0x040a, 0x028d), &[0xe9, 0x78, 0x00], Hook::FinishCombatInput),
    before(
        "sight shown with the scene",
        at(0x040a, 0x059f),
        &[0x83, 0x3e, 0x02, 0x53, 0x00],
        Hook::ShowCombatSight,
    ),
    before("fight ends", at(0x040a, 0x0768), POP_DI, Hook::EndCombat),
    before(
        "sight drawn at display cadence",
        at(0x040a, 0x08f1),
        LCALL_SIGHT_BLIT,
        Hook::DeferCombatSight,
    ),
    before(
        "sight drawn at display cadence",
        at(0x040a, 0x0e4f),
        LCALL_SIGHT_BLIT,
        Hook::DeferCombatSight,
    ),
    // Mining.
    before(
        "held Space keeps mining",
        at(0x0bb4, 0x1113),
        &[0x3d, 0x80, 0x00],
        Hook::ContinueMining,
    ),
    // Keyboard.
    before(
        "a key never replaces held directions",
        at(0x0fa7, 0x00dc),
        &[0x23, 0xff],
        Hook::KeepHeldDirections,
    ),
    // The walking command poll and its pointer.
    before(
        "pending click to the selector",
        at(0x0000, 0x07f5),
        &[0x83, 0x3e, 0x62, 0x5d, 0x00],
        Hook::DispatchPendingClick,
    ),
    before("keep the cave return position", at(0x0000, 0x0801), &[0xeb, 0x29], Hook::KeepScene),
    before(
        "latched click to the selector",
        at(0x0000, 0x084b),
        &[0x8d, 0x46, 0xfa],
        Hook::DispatchWorldClick,
    ),
    before("no selection keeps walking", at(0x0000, 0x08bf), &[0x23, 0xc0], Hook::KeepWalking),
    // Fast walking: town, saloon and mine loops.
    before(
        "walking step",
        at(0x0000, 0x029d),
        &[0x9a, 0x08, 0x00, 0xd6, 0x05],
        Hook::BeginWalkTick,
    ),
    before(
        "every other fast step keeps the clock",
        at(0x0000, 0x02bd),
        LCALL_WALK_CLOCK,
        Hook::SkipExtraWalkTick,
    ),
    before("walking step", at(0x08c0, 0x0acb), LCALL_WALK_CLOCK, Hook::BeginWalkTick),
    before(
        "every other fast step keeps the clock",
        at(0x08c0, 0x0acb),
        LCALL_WALK_CLOCK,
        Hook::SkipExtraWalkTick,
    ),
    before("walking step", at(0x0bb4, 0x069b), LCALL_WALK_CLOCK, Hook::BeginWalkTick),
    before(
        "every other fast step keeps the clock",
        at(0x0bb4, 0x069b),
        LCALL_WALK_CLOCK,
        Hook::SkipExtraWalkTick,
    ),
    before(
        "fast walking halves the delay",
        at(0x0000, 0x02b0),
        LCALL_WALK_DELAY,
        Hook::HalveWalkDelay,
    ),
    before(
        "fast walking halves the delay",
        at(0x08c0, 0x0b28),
        LCALL_WALK_DELAY,
        Hook::HalveWalkDelay,
    ),
    before(
        "fast walking halves the delay",
        at(0x0bb4, 0x0741),
        LCALL_WALK_DELAY,
        Hook::HalveWalkDelay,
    ),
    before(
        "every other fast mine step keeps the hazards",
        at(0x0bb4, 0x076d),
        &[0xb8, 0x30, 0x00],
        Hook::SkipExtraHazardCheck,
    ),
    // Mules.
    replace(
        "mule 1 sold cheapest first",
        at(0x08c0, 0x2235),
        &[0x83, 0x3e, 0x5a, 0x5d, 0x00],
        Hook::MuleForSale(0),
    ),
    replace(
        "mule 2 sold cheapest first",
        at(0x08c0, 0x2273),
        &[0x83, 0x3e, 0x5c, 0x5d, 0x00],
        Hook::MuleForSale(1),
    ),
    replace(
        "mule 3 sold cheapest first",
        at(0x08c0, 0x22b1),
        &[0x83, 0x3e, 0x5e, 0x5d, 0x00],
        Hook::MuleForSale(2),
    ),
    // The QoL panel and menus.
    before("panel artwork", at(0x0505, 0x096b), RETF, Hook::PreparePanel),
    before("scene context buttons", at(0x0505, 0x032e), PUSH_BP, Hook::ReadContextButtons),
    before(
        "scene context buttons cleared",
        at(0x0505, 0x0538),
        &[0x9a, 0x65, 0x00, 0x65, 0x12],
        Hook::ClearContextButtons,
    ),
    before("selector menu opens", at(0x0000, 0x093e), PUSH_BP, Hook::OpenSelectorMenu),
    before("selector menu closes", at(0x0000, 0x0a83), RETF, Hook::CloseSelectorMenu),
    before("building menu opens", at(0x0000, 0x0ae4), PUSH_BP, Hook::OpenBuildingMenu),
    before("building menu closes", at(0x0000, 0x0b6b), RETF, Hook::CloseBuildingMenu),
    before("saloon sleep begins", at(0x08c0, 0x29b6), &[0xb8, 0xa4, 0x09], Hook::BeginSleep),
    before("saloon sleep ends", at(0x08c0, 0x2a8b), RETF, Hook::EndSleep),
    before("mule shop opens", at(0x08c0, 0x1de2), PUSH_BP, Hook::EnterMuleShop),
    before("mule shop closes", at(0x08c0, 0x21b5), RETF, Hook::LeaveMuleShop),
    before("mule offers drawn", at(0x08c0, 0x0189), &[0xb8, 0x06, 0x00], Hook::ShowMuleShop),
    before("mule offers covered", at(0x08c0, 0x2065), &[0x83, 0xc4, 0x04], Hook::HideMuleShop),
    // Keyboard readers: only movement polls use native direction aliases.
    before("movement reader", at(0x0fa7, 0x009d), LCALL_KEY_READER, Hook::MovementKeys(true)),
    before("movement reader", at(0x0fa7, 0x00a4), LCALL_KEY_READER, Hook::MovementKeys(true)),
    before("other reader", at(0x0fa7, 0x00a2), &[0xeb, 0x0c], Hook::MovementKeys(false)),
    before("other reader", at(0x0fa7, 0x00a9), &[0xb9, 0x08, 0x00], Hook::MovementKeys(false)),
    before(
        "command poll reads directions",
        at(0x0000, 0x0882),
        LCALL_COMMAND_READER,
        Hook::MovementKeys(true),
    ),
    before(
        "command poll reads other keys",
        at(0x0000, 0x0887),
        &[0x89, 0x46, 0xf4],
        Hook::MovementKeys(false),
    ),
    before(
        "a direction leaves the hand",
        at(0x0000, 0x0887),
        &[0x89, 0x46, 0xf4],
        Hook::ReturnToWalking,
    ),
    before(
        "command poll reads directions",
        at(0x0000, 0x0a90),
        LCALL_COMMAND_READER,
        Hook::MovementKeys(true),
    ),
    before(
        "command poll reads other keys",
        at(0x0000, 0x0a95),
        &[0x89, 0x46, 0xfa],
        Hook::MovementKeys(false),
    ),
    // Readable routines.
    routine("the asset decoder", at(0x1265, 0x1250), &[0x8b, 0x04], Routine::DecodeAsset),
];
