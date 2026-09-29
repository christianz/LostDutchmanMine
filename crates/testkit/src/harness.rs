//! Driving the translated game step by step, as the C++ oracle's native tests
//! drove its generated code: no desktop and no quanta, only translated steps,
//! with a synthetic timer interrupt every thousand of them once the game runs.
//!
//! [`Harness::loaded`] leaves the image as DOS loaded it, for calling one
//! original routine on a hand-made stack. [`Harness::in_town`] runs the original
//! startup to the first town input, where every scene test begins. Tests then
//! set globals, [`call`](Harness::call) original routines and step
//! [`until`](Harness::until) a condition holds, reading the machine directly.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use dos::{Keyboard, MouseInput};
use engine::Program;
use game::symbols::{DATA_SEGMENT, Global, JOYSTICK, SCENE_TOWN, set_joystick};
use game::{Frame, Game, Report};
use machine::{Address, LOAD_SEGMENT, Machine};

mod fixture;
mod timer;

/// Translated steps between synthetic timer interrupts, as in the C++ tests.
const TIMER_PERIOD: u64 = 1000;
/// Startup, and any scene, finishes well within this many steps.
pub const SCENE_LIMIT: u64 = 120_000_000;
/// Where [`Harness::call`] returns to, far from any translated code.
pub const RETURN_SEGMENT: u16 = 0xffff;
/// See [`RETURN_SEGMENT`].
pub const RETURN_OFFSET: u16 = 0xfffe;
/// The empty stack a call starts from, well clear of the game's data.
pub const STACK_TOP: u16 = 0x8000;
/// The game's keyboard and joystick poll: every scene's input point.
pub const INPUT_POLL: Address = Address::new(0x0fa7, 0x0006);

/// The original's mouse-driver flag: 2 once a driver is found.
const MOUSE_DRIVER: Global = Global(0x0a64);
/// A byte set when the original's BIOS wrappers take far returns.
const FAR_WRAPPERS: u16 = 0x0aa6;
/// The original movement reader's own held-direction byte.
const HELD_DIRECTIONS: Global = Global(0x0a6c);
/// The segment of the joystick port the movement reader reads.
const JOYSTICK_SEGMENT: Global = Global(0x3118);

/// Distinguishes the saves folders of harnesses running at once.
static FOLDERS: AtomicUsize = AtomicUsize::new(0);

/// The workspace this crate belongs to.
pub fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The original game's folder: `LDM_DATA`, or `.local/original` in the workspace.
pub fn data() -> PathBuf {
    std::env::var_os("LDM_DATA").map_or_else(|| workspace().join(".local/original"), PathBuf::from)
}

/// The translated game on its machine, stepped one translated run at a time.
#[derive(Debug)]
pub struct Harness {
    /// The machine.
    pub m: Machine,
    /// The game beside it: DOS, and the port's own state.
    pub game: Game,
    /// The step count at which the next timer interrupt falls due, once the
    /// clock runs.
    next_timer: Option<u64>,
    /// Timer interrupts that fell due, delivered or masked, as the C++ tests
    /// counted them.
    pub timers: u64,
    /// A code address whose steps are counted, and the count.
    watched: Option<(Address, u64)>,
    /// A fresh folder for saves, removed with the harness.
    saves: PathBuf,
}

impl Harness {
    /// The game as DOS loaded it, not started: the data segment in DS and SS
    /// and an empty stack at [`STACK_TOP`], ready to call one original routine.
    /// No timer interrupts arrive. QoL is on, as the C++ `State` began.
    ///
    /// # Panics
    ///
    /// When the game does not boot: see [`game::boot`].
    pub fn loaded() -> Self {
        let mut h = Harness::boot();
        let data = DATA_SEGMENT + LOAD_SEGMENT;
        (h.m.regs.ds, h.m.regs.ss, h.m.regs.sp) = (data, data, STACK_TOP);
        h
    }

    /// The game started with the synthetic clock and run to its first town
    /// input, with its original initialisation, assets and timer handler.
    ///
    /// # Panics
    ///
    /// When the game does not boot or never reaches the town.
    pub fn in_town() -> Self {
        let mut h = Harness::boot();
        h.next_timer = Some(TIMER_PERIOD);
        h.until(SCENE_LIMIT, "Original startup did not reach town", |h| {
            h.at(INPUT_POLL) && SCENE_TOWN.at_rest(&h.m) == 1
        });
        h
    }

    /// The game as DOS loaded it, saving to a fresh folder of its own.
    fn boot() -> Self {
        let folder = FOLDERS.fetch_add(1, Ordering::Relaxed);
        let saves =
            std::env::temp_dir().join(format!("ldm-native-{}-{folder}", std::process::id()));
        if saves.exists() {
            std::fs::remove_dir_all(&saves).expect("a stale saves folder can be removed");
        }
        std::fs::create_dir_all(&saves).expect("a fresh saves folder");
        let (m, game) = game::boot(data(), saves.clone(), true)
            .expect("the game boots: build with LDM_EXE and set LDM_DATA to its folder");
        Harness { m, game, next_timer: None, timers: 0, watched: None, saves }
    }

    /// One translated step, then the timer interrupt if one falls due.
    ///
    /// # Panics
    ///
    /// When the game stops: a fault, untranslated code or a failed service.
    pub fn step(&mut self) {
        let at = Address::from_runtime(self.m.regs.cs, self.m.regs.ip);
        if let Some((address, visits)) = &mut self.watched
            && *address == at
        {
            *visits += 1;
        }
        if let Err(stop) = self.game.step(&mut self.m) {
            panic!("the translated game stopped in a step from {at}: {stop}");
        }
        if let Some(due) = self.next_timer
            && self.m.steps >= due
        {
            self.next_timer = Some(self.m.steps + TIMER_PERIOD);
            self.timers += 1;
            timer::interrupt(&mut self.m, &mut self.game);
        }
    }

    /// Steps until `done` holds, which it may already.
    ///
    /// # Panics
    ///
    /// With `failure` when `done` still fails after `limit` steps.
    pub fn until(&mut self, limit: u64, failure: &str, mut done: impl FnMut(&Self) -> bool) {
        let start = self.m.steps;
        while !done(self) {
            assert!(self.m.steps - start < limit, "{failure}");
            self.step();
        }
    }

    /// Whether the next step starts at `address`.
    pub fn at(&self, address: Address) -> bool {
        self.m.regs.cs == address.runtime_segment() && self.m.regs.ip == address.offset
    }

    /// Counts the steps that start at `address` from now on.
    pub fn watch(&mut self, address: Address) {
        self.watched = Some((address, 0));
    }

    /// The steps that started at the watched address.
    pub fn visits(&self) -> u64 {
        self.watched.map_or(0, |(_, visits)| visits)
    }

    /// Runs `run` with the synthetic clock stopped: no timer interrupt falls
    /// due until it returns, as when the C++ tests stepped generated code
    /// directly.
    pub fn with_clock_stopped<R>(&mut self, run: impl FnOnce(&mut Self) -> R) -> R {
        let clock = self.next_timer.take();
        let result = run(self);
        self.next_timer = clock;
        result
    }

    /// Far-calls `routine` on an empty stack at [`STACK_TOP`] with `args` in
    /// the order the routine declares them, so the first ends on top.
    pub fn call(&mut self, routine: Address, args: &[u16]) {
        self.call_with_stack(STACK_TOP, routine, args);
    }

    /// Far-calls `routine` as [`Harness::call`] does, on a stack at `top`:
    /// clear of a suspended scene's own stack, or on top of it.
    pub fn call_with_stack(&mut self, top: u16, routine: Address, args: &[u16]) {
        self.m.regs.sp = top;
        for &arg in args.iter().rev() {
            self.m.push(arg);
        }
        self.m.push(RETURN_SEGMENT);
        self.m.push(RETURN_OFFSET);
        self.jump(routine);
    }

    /// Continues at `address`.
    pub fn jump(&mut self, address: Address) {
        (self.m.regs.cs, self.m.regs.ip) = (address.runtime_segment(), address.offset);
    }

    /// Whether a call has returned: nothing runs at [`RETURN_SEGMENT`].
    pub fn returned(&self) -> bool {
        self.m.regs.cs == RETURN_SEGMENT
    }

    /// Steps until the current call returns.
    ///
    /// # Panics
    ///
    /// When it does not return within [`SCENE_LIMIT`] steps, or returns
    /// somewhere other than [`RETURN_OFFSET`].
    pub fn finish(&mut self) {
        self.until(SCENE_LIMIT, "Original routine did not return", Harness::returned);
        assert_eq!(self.m.regs.ip, RETURN_OFFSET, "Corrupted original far return");
    }

    /// A global, read through DS as the game's code reads it.
    pub fn get(&self, global: Global) -> u16 {
        global.get(&self.m)
    }

    /// Writes a global through DS.
    pub fn set(&mut self, global: Global, value: u16) {
        global.set(&mut self.m, value);
    }

    /// The BIOS keyboard buffer.
    pub fn keyboard(&mut self) -> &mut Keyboard {
        &mut self.game.dos.keyboard
    }

    /// The desktop's mouse input.
    pub fn mouse(&mut self) -> &mut MouseInput {
        &mut self.game.dos.mouse.input
    }

    /// Holds the movement directions on the joystick port the original reads.
    pub fn set_movement(&mut self, directions: u8) {
        set_joystick(&mut self.m, directions);
    }

    /// Tells the original's input wrappers that a mouse driver is present and
    /// that they take far returns, as the C++ tests did in place of startup.
    pub fn install_mouse_driver(&mut self) {
        self.set(MOUSE_DRIVER, 2);
        self.m.memory.write8(self.m.regs.ds, FAR_WRAPPERS, 1);
    }

    /// Points the original movement reader at the joystick port the port holds
    /// directions on, its own held-direction byte clear, as the C++ tests did
    /// in place of startup.
    pub fn attach_joystick(&mut self) {
        self.set(HELD_DIRECTIONS, 0);
        self.set(JOYSTICK_SEGMENT, JOYSTICK.runtime_segment());
    }

    /// Forgets queued keys, clicks, the walking pointer's baseline and held
    /// directions, as the C++ tests did before driving a scene.
    pub fn forget_input(&mut self) {
        self.game.dos.keyboard.clear();
        self.game.dos.mouse.input.clear();
        self.game.reset_world_pointer();
        self.set_movement(0);
    }

    /// The frame the player sees now.
    pub fn frame(&self) -> Frame {
        self.game.frame(&self.m)
    }

    /// The game's state summary now.
    pub fn report(&self) -> Report {
        self.game.report(&self.m)
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        // A folder left behind in the temporary directory harms nothing.
        let _ = std::fs::remove_dir_all(&self.saves);
    }
}
