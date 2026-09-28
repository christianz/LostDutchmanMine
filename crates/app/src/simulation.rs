//! The game on its own thread, one emulated millisecond per wall-clock
//! millisecond.
//!
//! A slow present, a busy monitor or an open settings menu must never change
//! the cadence of the game's own timer, so the window never runs the game:
//! it sends input here and shows the latest frame published from here.
//! Pausing stops emulated time; it does not catch up afterwards.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use engine::{Engine, InputKind};
use game::{Frame, Game};

/// At most this many milliseconds are run at once; after a longer stall the
/// game resumes where it was rather than racing to catch up.
const MAX_CATCH_UP: u64 = 50;
/// How often a new frame is published.
const PUBLISH_EVERY: Duration = Duration::from_millis(4);

/// A change for the sound output, in the order the game made it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    /// An OPL port write: offset and value.
    Opl(u16, u8),
    /// The PC speaker's frequency, or zero for silence.
    Speaker(u32),
}

/// What the window shows.
#[derive(Clone, Debug)]
pub struct Snapshot {
    /// The game's frame.
    pub frame: Frame,
    /// The BIOS video mode.
    pub video_mode: u8,
    /// Whether the game draws its own pointer, so the system's should hide.
    pub custom_cursor: bool,
    /// Whether the game has exited.
    pub finished: bool,
    /// Why it stopped, if it failed.
    pub error: Option<String>,
}

#[derive(Default)]
struct Shared {
    inputs: Mutex<Vec<InputKind>>,
    latest: Mutex<Option<Snapshot>>,
    sound: Mutex<VecDeque<Sound>>,
    paused: AtomicBool,
    stopping: AtomicBool,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The running game.
pub struct Simulation {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl Simulation {
    /// Starts `engine` on its own thread.
    pub fn start(engine: Engine<Game>) -> Self {
        let shared = Arc::new(Shared::default());
        let thread = std::thread::Builder::new()
            .name("simulation".to_owned())
            .spawn({
                let shared = Arc::clone(&shared);
                move || run(engine, &shared)
            })
            .expect("a thread for the simulation");
        Simulation { shared, thread: Some(thread) }
    }

    /// Queues input for the next millisecond.
    pub fn send(&self, inputs: impl IntoIterator<Item = InputKind>) {
        lock(&self.shared.inputs).extend(inputs);
    }

    /// Pauses or resumes emulated time.
    pub fn pause(&self, paused: bool) {
        self.shared.paused.store(paused, Ordering::Relaxed);
    }

    /// The latest snapshot, once the first has been published.
    pub fn latest(&self) -> Option<Snapshot> {
        lock(&self.shared.latest).clone()
    }

    /// Sound changes made since the last call.
    pub fn take_sound(&self) -> VecDeque<Sound> {
        std::mem::take(&mut *lock(&self.shared.sound))
    }
}

impl Drop for Simulation {
    fn drop(&mut self) {
        self.shared.stopping.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(mut engine: Engine<Game>, shared: &Shared) {
    let mut emulated = Duration::ZERO;
    let mut last = Instant::now();
    let mut published: Option<Instant> = None;
    let mut speaker = 0;
    let mut error = None;
    while !shared.stopping.load(Ordering::Relaxed) {
        let now = Instant::now();
        let elapsed = now - last;
        last = now;
        if !shared.paused.load(Ordering::Relaxed) {
            emulated += elapsed;
            for input in std::mem::take(&mut *lock(&shared.inputs)) {
                engine.input(input);
            }
            let target = emulated.as_millis() as u64;
            let behind = target.saturating_sub(engine.quanta());
            for _ in 0..behind.min(MAX_CATCH_UP) {
                if let Err(stop) = engine.step() {
                    error = Some(stop.to_string());
                    break;
                }
                let audio = engine.take_audio();
                let mut sound = lock(&shared.sound);
                sound.extend(
                    audio.opl_writes.into_iter().map(|(offset, value)| Sound::Opl(offset, value)),
                );
                if audio.speaker_hz != speaker {
                    speaker = audio.speaker_hz;
                    sound.push_back(Sound::Speaker(speaker));
                }
            }
            if behind > MAX_CATCH_UP {
                emulated = Duration::from_millis(engine.quanta());
            }
        }
        let finished = engine.finished() || error.is_some();
        if finished || published.is_none_or(|at| now - at >= PUBLISH_EVERY) {
            let (m, game) = (engine.machine(), engine.program());
            *lock(&shared.latest) = Some(Snapshot {
                frame: game.frame(m),
                video_mode: game.dos.video.mode,
                custom_cursor: game.dos.mouse.custom_cursor,
                finished,
                error: error.clone(),
            });
            published = Some(now);
        }
        if finished {
            return;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
