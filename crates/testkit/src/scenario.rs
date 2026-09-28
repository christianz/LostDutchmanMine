//! Scenario runs: the game booted in a fresh folder, driven by a script through
//! the desktop controller, one emulated millisecond at a time, exactly as the
//! C++ desktop ran its deterministic `--trace` runs.

use std::path::{Path, PathBuf};

use desktop::controller::Controller;
use desktop::keys::{Key, SHIFT, bios, default_keycode};
use desktop::settings::{self, DisplaySettings};
use engine::{Engine, InputKind, Stop};
use game::{BootError, Frame, Game, Report, RoutineMode};
use serde::Deserialize;

use crate::script::{self, Action, ScriptError};

/// A trace line is written every this many emulated milliseconds.
const TRACE_INTERVAL: u64 = 100;

/// One scenario from `tests/scenarios.json`.
#[derive(Clone, Debug, Deserialize)]
pub struct Scenario {
    /// Its name, also the golden trace's.
    pub name: String,
    /// The script in `tests/scripts`.
    pub script: String,
    /// How long it runs, in emulated seconds.
    pub seconds: u64,
    /// The saved game it starts from, built by `tools/fixtures.py`.
    #[serde(default)]
    pub fixture: Option<String>,
    /// Whether the settings menu opens at launch.
    #[serde(default)]
    pub settings: bool,
    /// The QoL setting, written to the settings file with the menu off.
    #[serde(default)]
    pub qol: Option<u8>,
}

/// The scenario manifest.
#[derive(Clone, Debug, Deserialize)]
pub struct Manifest {
    /// Every scenario.
    pub scenarios: Vec<Scenario>,
}

impl Manifest {
    /// Reads `tests/scenarios.json` under the workspace `root`.
    ///
    /// # Errors
    ///
    /// [`RunError`] when it cannot be read or parsed.
    pub fn load(root: &Path) -> Result<Self, RunError> {
        let path = root.join("tests/scenarios.json");
        let text =
            std::fs::read_to_string(&path).map_err(|source| RunError::Io { path, source })?;
        serde_json::from_str(&text).map_err(|error| RunError::Manifest(error.to_string()))
    }
}

/// The game's frame and state at a `capture` event.
#[derive(Clone, Debug)]
pub struct Capture {
    /// The capture's number in the script.
    pub id: u32,
    /// What the game showed.
    pub frame: Frame,
    /// Its state.
    pub report: Report,
}

/// The game's frame and the desktop's view of it at a `screen` event.
#[derive(Clone, Debug)]
pub struct Screen {
    /// The capture's number in the script.
    pub id: u32,
    /// What the game showed.
    pub frame: Frame,
    /// The settings the picture followed.
    pub settings: DisplaySettings,
    /// Whether the settings menu covered it.
    pub menu_open: bool,
}

/// How a run ended: what the C++ desktop logged as the game switched video
/// modes and when it closed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ending {
    /// Emulated milliseconds from launch to the end, pauses included.
    pub ms: u64,
    /// Whether the game exited by itself, rather than the run running out of
    /// time or the desktop quitting.
    pub exited: bool,
    /// Every video mode the game switched to, in order. Modes are sampled each
    /// millisecond, so a switch undone within one is not seen.
    pub video_modes: Vec<u8>,
    /// The BIOS video mode at the end.
    pub video_mode: u8,
    /// The PIT's timer divisor at the end; zero is the BIOS default, 65536.
    pub pit_divisor: u16,
}

impl Ending {
    /// Notes the video mode after a millisecond.
    fn note(&mut self, mode: u8) {
        if mode != self.video_mode {
            self.video_modes.push(mode);
            self.video_mode = mode;
        }
    }
}

/// What a run produced.
#[derive(Clone, Debug, Default)]
pub struct Outcome {
    /// The trace lines, as the oracle writes them.
    pub trace: Vec<String>,
    /// Every `capture`, in script order.
    pub captures: Vec<Capture>,
    /// Every `screen`, in script order.
    pub screens: Vec<Screen>,
    /// How it ended.
    pub ending: Ending,
    /// Readable routine calls checked in lockstep with their translation.
    pub routine_checks: u64,
    /// What those checks found to differ.
    pub routine_mismatches: Vec<String>,
}

/// Why a scenario could not run.
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// A file could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// The file.
        path: PathBuf,
        /// The failure.
        source: std::io::Error,
    },
    /// The manifest is malformed.
    #[error("tests/scenarios.json: {0}")]
    Manifest(String),
    /// The script is malformed.
    #[error("{script}: {error}")]
    Script {
        /// The script.
        script: String,
        /// Where and why.
        error: ScriptError,
    },
    /// The game did not start.
    #[error(transparent)]
    Boot(#[from] BootError),
    /// The game stopped.
    #[error("at {ms} ms: {stop}")]
    Stopped {
        /// When.
        ms: u64,
        /// Why.
        stop: Stop,
    },
}

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> RunError + '_ {
    move |source| RunError::Io { path: path.to_path_buf(), source }
}

/// Runs `scenario` from the workspace `root` with the original game in `data`,
/// in a fresh `folder` that receives its saves and settings file.
///
/// # Errors
///
/// [`RunError`] when the scenario cannot be prepared, or the game stops.
pub fn run(
    scenario: &Scenario,
    root: &Path,
    data: &Path,
    folder: &Path,
) -> Result<Outcome, RunError> {
    let script_path = root.join("tests/scripts").join(&scenario.script);
    let text = std::fs::read_to_string(&script_path).map_err(io(&script_path))?;
    let script = script::parse(&text)
        .map_err(|error| RunError::Script { script: scenario.script.clone(), error })?;
    let saves = prepare(scenario, root, folder)?;
    let config = folder.join("display.ini");
    if let Some(qol) = scenario.qol {
        std::fs::write(&config, format!("qol={qol}\nstartup=0\n")).map_err(io(&config))?;
    }
    let settings = settings::load(&config);
    let (machine, mut game) = game::boot(data.to_path_buf(), saves, settings.qol)?;
    // The translation keeps the oracle's timing, and every readable routine
    // call is checked against it on the side.
    game.set_routine_mode(RoutineMode::Lockstep);
    let mut controller = Controller::new(config, settings, scenario.settings);
    let mut engine = Engine::new(machine, game);
    let mut outcome = Outcome::default();
    outcome.ending.video_mode = engine.program().dos.video.mode;
    let mut events = script.into_iter().peekable();
    let mut traced = 0;
    let mut end = scenario.seconds * 1000;
    for now in 0..scenario.seconds * 1000 {
        if engine.finished() {
            end = now;
            break;
        }
        controller.tick(now, engine.program().dos.video.mode);
        while let Some(event) = events.next_if(|event| event.at_ms <= now) {
            act(event.action, &mut controller, &engine, &mut outcome);
        }
        for input in controller.take_inputs() {
            engine.input(input);
        }
        if !controller.paused() {
            engine.step().map_err(|stop| RunError::Stopped { ms: now, stop })?;
            if engine.quanta() % TRACE_INTERVAL == 0 {
                outcome.trace.push(engine.trace_line());
                traced = engine.quanta();
            }
            outcome.ending.note(engine.program().dos.video.mode);
        }
        if controller.quit() {
            end = now + 1;
            break;
        }
    }
    if traced != engine.quanta() {
        outcome.trace.push(engine.trace_line());
    }
    outcome.ending.ms = end;
    outcome.ending.exited = engine.finished();
    outcome.ending.pit_divisor = engine.machine().pit.timer_divisor;
    outcome.routine_checks = engine.program().routine_checks();
    outcome.routine_mismatches = engine.program().routine_mismatches().to_vec();
    Ok(outcome)
}

/// A fresh folder, with the fixture's saves or none.
fn prepare(scenario: &Scenario, root: &Path, folder: &Path) -> Result<PathBuf, RunError> {
    if folder.exists() {
        std::fs::remove_dir_all(folder).map_err(io(folder))?;
    }
    let saves = folder.join("Saves");
    std::fs::create_dir_all(&saves).map_err(io(&saves))?;
    if let Some(fixture) = &scenario.fixture {
        copy_folder(&root.join(".local/fixtures").join(fixture), &saves)?;
    }
    Ok(saves)
}

fn copy_folder(from: &Path, to: &Path) -> Result<(), RunError> {
    for entry in std::fs::read_dir(from).map_err(io(from))? {
        let path = entry.map_err(io(from))?.path();
        let target = to.join(path.file_name().expect("a directory entry has a name"));
        if path.is_dir() {
            std::fs::create_dir_all(&target).map_err(io(&target))?;
            copy_folder(&path, &target)?;
        } else {
            std::fs::copy(&path, &target).map_err(io(&path))?;
        }
    }
    Ok(())
}

fn act(action: Action, controller: &mut Controller, engine: &Engine<Game>, outcome: &mut Outcome) {
    let physical =
        |scancode, mods, repeat| Key { scancode, keycode: default_keycode(scancode), mods, repeat };
    match action {
        Action::Key(key) => controller.send(InputKind::Key(u32::from(key))),
        Action::Down { scancode, mods } => controller.key(physical(scancode, mods, false), true),
        Action::Repeat { scancode, mods } => controller.key(physical(scancode, mods, true), true),
        Action::Up { scancode, mods } => controller.key(physical(scancode, mods, false), false),
        Action::FocusLost => controller.focus_lost(),
        Action::Ascii(character) => {
            let (character, mods) = if character.is_ascii_uppercase() {
                (character.to_ascii_lowercase(), SHIFT)
            } else {
                (character, 0)
            };
            let key = Key { scancode: 0, keycode: u32::from(character), mods, repeat: false };
            controller.send(InputKind::Key(u32::from(bios(key))));
        }
        Action::Mouse { x, y } => controller.send(InputKind::Mouse { x, y }),
        Action::Buttons(buttons) => controller.send(InputKind::Buttons(buttons)),
        Action::Screen(id) => outcome.screens.push(Screen {
            id,
            frame: engine.program().frame(engine.machine()),
            settings: *controller.preview(),
            menu_open: controller.menu_open(),
        }),
        Action::Capture(id) => {
            let (m, game) = (engine.machine(), engine.program());
            outcome.captures.push(Capture { id, frame: game.frame(m), report: game.report(m) });
        }
    }
}
