//! The hooks patches run, and what may follow them.

/// What follows a hook.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum After {
    /// Carry on: with the instruction (`Before`) or after it (`Replace`).
    Continue,
    /// Run the original instruction after all (`Replace` only).
    Original,
    /// Continue at an offset in this segment, one of [`Hook::targets`].
    Goto(u16),
    /// Stop this run at the patched instruction; it runs again next time.
    Yield,
    /// Stop this run and resume at an offset in this segment.
    YieldAt(u16),
}

/// A hook: game behaviour at a patched instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hook {
    /// The Space desert close-up waits for its own dismissal (QoL), instead of a
    /// timed preview that let a held Space open another one.
    DesertCloseUp,
    /// Leaving a loaded saloon returns to its saved doorway.
    SaloonReturnPosition,
    /// Restores the panning reward loop the supplied executable jumps over.
    PanningRewardLoop,
    /// Restores the panning animation frames the supplied executable skips.
    PanningFrameLoop,
    /// Chooses VGA without showing the original graphics selector.
    SkipGraphicsSelector,
    /// Pan only with a pan actually carried by the player or an owned mule.
    PanOwnership,
    /// Mules are sold cheapest first with QoL (mule 0, 1 or 2).
    MuleForSale(u8),
    /// Latches one desktop mouse sample for the whole original mouse read.
    MousePoll,
    /// Starts the panning activity, draining input.
    BeginPanning,
    /// Ends the panning activity, draining input.
    FinishPanning,
    /// A fight starts: aim immediately with QoL when armed.
    BeginCombat,
    /// The encounter's aiming read begins.
    BeginCombatInput,
    /// Walking and the pointer coexist with QoL; latches fresh clicks.
    FilterWorldMouse,
    /// A left press in the shooting area fires instead of opening the hand.
    FilterCombatMouse,
    /// The encounter's aiming read ends: apply mouse aim and shots.
    FinishCombatInput,
    /// The sight is shown once the encounter draws its scene.
    ShowCombatSight,
    /// The fight is over.
    EndCombat,
    /// With QoL the sight is drawn at display cadence, not by this blit.
    DeferCombatSight,
    /// Holding Space keeps the pick swinging at its original stroke rate.
    ContinueMining,
    /// A single key's scan never replaces the combined held directions.
    KeepHeldDirections,
    /// A pending QoL click goes straight to the original selector.
    DispatchPendingClick,
    /// Selector result 9 restarts the scene only for the walking poll's own
    /// hand; with QoL it would lose the cave return position.
    KeepScene,
    /// Dispatches a latched QoL click through the original selector.
    DispatchWorldClick,
    /// With QoL, no selection means keep walking instead of entering the hand.
    KeepWalking,
    /// The original selector's setup: the physical pointer stays put.
    ResetWorldPointer,
    /// A walking step begins: twice as many steps while a direction is held.
    BeginWalkTick,
    /// Every other fast step skips the survival clock and hazard update.
    SkipExtraWalkTick,
    /// Fast walking halves the original per-step delay.
    HalveWalkDelay,
    /// Every other fast mine step skips the hazard check.
    SkipExtraHazardCheck,
    /// The original panel is on screen: load its artwork for the QoL toolbar.
    PreparePanel,
    /// Records which context buttons the current scene offers.
    ReadContextButtons,
    /// The scene's context buttons are cleared.
    ClearContextButtons,
    /// The shared selector's hook: remap QoL toolbar clicks, open a menu.
    OpenSelectorMenu,
    /// A menu opened from the walking poll closes.
    CloseSelectorMenu,
    /// A building's menu opens.
    OpenBuildingMenu,
    /// A building's menu closes.
    CloseBuildingMenu,
    /// The noninteractive saloon sleep begins: no hover underneath.
    BeginSleep,
    /// The saloon sleep ends.
    EndSleep,
    /// The mule shop opens.
    EnterMuleShop,
    /// The mule shop closes.
    LeaveMuleShop,
    /// The mule shop's offers are drawn.
    ShowMuleShop,
    /// The mule shop's offers are hidden by a dialog.
    HideMuleShop,
    /// A movement reader (true) or another reader (false) polls the keyboard.
    MovementKeys(bool),
    /// In the hand-cursor loop a direction returns to walking.
    ReturnToWalking,
}

/// Where hooks continue, by the original code's meaning. Offsets are in the
/// segment of the hook's site.
pub mod resume {
    /// 033f: the panning reward loop's body, restored.
    pub const PANNING_REWARD_LOOP: u16 = 0x0308;
    /// 033f: after the panning reward loop.
    pub const AFTER_PANNING_REWARD: u16 = 0x0410;
    /// 033f: the panning frame loop's comparison.
    pub const PANNING_FRAME_TEST: u16 = 0x03a0;
    /// 033f: after the panning frame loop.
    pub const AFTER_PANNING_FRAMES: u16 = 0x03a3;
    /// 1265: the accepted-VGA path of the graphics selector.
    pub const VGA_CHOSEN: u16 = 0x06f7;
    /// 0000: the walking poll's call of the selector.
    pub const SELECTOR_CALL: u16 = 0x07fc;
    /// 0000: the walking poll's handling of a selection.
    pub const SELECTION: u16 = 0x08cc;
    /// 0000: the walking poll's return to keyboard walking.
    pub const WALKING: u16 = 0x0914;
    /// 0bb4: the mine loop after its hazard check.
    pub const AFTER_HAZARD_CHECK: u16 = 0x07a6;
}

impl Hook {
    /// The offsets a hook may continue or resume at, given the offset of the
    /// instruction after its site. The translator makes each one a block start.
    pub fn targets(self, next: u16) -> Vec<u16> {
        use resume::{
            AFTER_HAZARD_CHECK, AFTER_PANNING_FRAMES, AFTER_PANNING_REWARD, PANNING_FRAME_TEST,
            PANNING_REWARD_LOOP, SELECTION, SELECTOR_CALL, VGA_CHOSEN, WALKING,
        };
        match self {
            Hook::DesertCloseUp | Hook::DeferCombatSight | Hook::SkipExtraWalkTick => vec![next],
            Hook::PanningRewardLoop => vec![PANNING_REWARD_LOOP, AFTER_PANNING_REWARD],
            Hook::PanningFrameLoop => vec![AFTER_PANNING_FRAMES, PANNING_FRAME_TEST],
            Hook::SkipGraphicsSelector => vec![VGA_CHOSEN],
            Hook::DispatchPendingClick => vec![SELECTOR_CALL],
            Hook::DispatchWorldClick => vec![SELECTION],
            Hook::KeepWalking | Hook::ReturnToWalking => vec![WALKING],
            Hook::SkipExtraHazardCheck => vec![AFTER_HAZARD_CHECK],
            _ => Vec::new(),
        }
    }
}
