//! Reaching into the port's own state, for tests and tools.
//!
//! The C++ oracle's native tests set and read its `State` directly: they
//! entered an encounter, forgot a pointer, covered the scene with a menu or
//! asked whether a click waited. These accessors give Rust tests and tools the
//! same reach, each a small read or action on the state the hooks keep beside
//! the machine. The game itself never calls them.
//!
//! Where a hook keeps its state to itself, the accessor asks the hook what it
//! would decide, on a copy of the machine, so the machine is never touched.

use engine::Stop;
use machine::Machine;
use patches::{After, Hook};

use crate::symbols::MULES_OWNED;
use crate::{Game, hooks, overlay};

/// The offset after a hook's instruction. These hooks run outside the game's
/// code, so none follows; of a jump only the decision to take it is read.
const NO_INSTRUCTION: u16 = 0;
/// A delay the walking hook halves on a fast step and leaves alone otherwise.
const PROBE_DELAY: u16 = 2;

impl Game {
    /// Sets the QoL flag alone, as the C++ tests did. The classic cursor
    /// parking follows it, but unlike a desktop toggle no pointer is baselined
    /// again.
    pub fn set_qol_flag(&mut self, on: bool) {
        self.qol = on;
        self.dos.park_cursor = !on;
    }

    /// Whether an encounter is running, as the hooks see it.
    pub fn combat_active(&self) -> bool {
        self.combat.active
    }

    /// Marks an encounter as running or not, without entering one.
    pub fn set_combat_active(&mut self, active: bool) {
        self.combat.active = active;
    }

    /// Enters an encounter as the original's entry hook does: with QoL and a
    /// gun, aiming starts at once and the click that entered is no shot.
    ///
    /// # Errors
    ///
    /// The hook's [`Stop`], which entering an encounter never produces.
    pub fn begin_combat(&mut self, m: &mut Machine) -> Result<(), Stop> {
        hooks::run(Hook::BeginCombat, m, self, NO_INSTRUCTION).map(drop)
    }

    /// Baselines encounter aiming on the pointer as it is now: only motion from
    /// here moves the sight, and a press made before is no shot.
    pub fn reset_combat_pointer(&mut self) {
        self.combat.reset_pointer(self.dos.mouse.input.current());
    }

    /// Baselines the walking pointer on the pointer as it is now and forgets a
    /// latched click, as the selector's setup does.
    pub fn reset_world_pointer(&mut self) {
        self.pointer.reset(self.dos.mouse.input.current());
    }

    /// With QoL, whether a click latched while walking waits for the selector.
    pub fn world_click_pending(&mut self, m: &Machine) -> bool {
        matches!(self.probe(Hook::DispatchPendingClick, &mut m.clone()), Ok(After::Goto(_)))
    }

    /// Whether the current walking step is fast, so its delay is halved.
    pub fn walk_is_fast(&mut self, m: &Machine) -> bool {
        let mut copy = m.clone();
        copy.push(PROBE_DELAY);
        self.probe(Hook::HalveWalkDelay, &mut copy).is_ok() && copy.pop() < PROBE_DELAY
    }

    /// Whether the current walking step is a fast walk's extra one, which
    /// leaves the survival clock and the mine's hazards alone.
    pub fn walk_step_is_extra(&mut self, m: &Machine) -> bool {
        matches!(self.probe(Hook::SkipExtraWalkTick, &mut m.clone()), Ok(After::Goto(_)))
    }

    /// Which of the scene's four action buttons the overlay offers: bit `i`
    /// for button `i`.
    pub fn context_buttons(&self) -> u8 {
        self.overlay.context_buttons
    }

    /// Offers the scene's action buttons, as its drawing routine would.
    pub fn set_context_buttons(&mut self, buttons: u8) {
        self.overlay.context_buttons = buttons;
    }

    /// A menu covers the scene: its buttons lose their hover targets.
    pub fn open_menu(&mut self) {
        overlay::open_menu(self);
    }

    /// The innermost menu closes, and what it covered returns.
    pub fn close_menu(&mut self) {
        overlay::close_menu(self);
    }

    /// Whether the mule shop's offers are on screen, marked where sold out.
    pub fn mule_shop_visible(&self) -> bool {
        self.overlay.mule_shop_visible
    }

    /// Whether mule `index` is for sale: not owned, and with QoL only once
    /// every cheaper mule is sold. Ownership is read between runs.
    pub fn mule_available(&self, m: &Machine, index: u8) -> bool {
        hooks::mule_available(self, index, |i| MULES_OWNED.nth(u16::from(i)).at_rest(m) != 0)
    }

    /// Holds or releases Space for mining, as the desktop's held key does.
    pub fn set_mining_space_held(&mut self, held: bool) {
        self.supplies.mining_space_held = held;
    }

    /// Asks `hook` what it decides on `m`, a copy of the machine.
    fn probe(&mut self, hook: Hook, m: &mut Machine) -> Result<After, Stop> {
        hooks::run(hook, m, self, NO_INSTRUCTION)
    }
}
