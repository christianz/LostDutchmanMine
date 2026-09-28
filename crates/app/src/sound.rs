//! The sound device: the game's AdLib and speaker, synthesised as it plays.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, PoisonError};

use desktop::audio::{SAMPLE_RATE, Synth};
use sdl2::AudioSubsystem;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};

use crate::simulation::Sound;

/// Samples per device callback: about ten milliseconds.
const BUFFER: u16 = 512;

struct Output {
    synth: Synth,
    changes: Arc<Mutex<VecDeque<Sound>>>,
}

impl AudioCallback for Output {
    type Channel = f32;

    fn callback(&mut self, samples: &mut [f32]) {
        let changes =
            std::mem::take(&mut *self.changes.lock().unwrap_or_else(PoisonError::into_inner));
        for change in changes {
            match change {
                Sound::Opl(offset, value) => self.synth.write(offset, value),
                Sound::Speaker(hz) => self.synth.speaker(hz),
            }
        }
        self.synth.render(samples);
    }
}

/// The open sound device, or none when the system has none.
pub struct Speakers {
    device: Option<AudioDevice<Output>>,
    changes: Arc<Mutex<VecDeque<Sound>>>,
}

impl Speakers {
    /// Opens the default device, paused. Without one the game plays silently.
    pub fn open(audio: Option<&AudioSubsystem>) -> Self {
        let changes = Arc::new(Mutex::new(VecDeque::new()));
        let desired = AudioSpecDesired {
            freq: Some(SAMPLE_RATE as i32),
            channels: Some(1),
            samples: Some(BUFFER),
        };
        let device = audio.and_then(|audio| {
            audio
                .open_playback(None, &desired, |_| Output {
                    synth: Synth::new(),
                    changes: Arc::clone(&changes),
                })
                .map_err(|error| eprintln!("no sound: {error}"))
                .ok()
        });
        Speakers { device, changes }
    }

    /// Passes the game's latest sound changes on.
    pub fn play(&self, changes: VecDeque<Sound>) {
        if self.device.is_some() {
            self.changes.lock().unwrap_or_else(PoisonError::into_inner).extend(changes);
        }
    }

    /// Plays or pauses, following the game.
    pub fn pause(&self, paused: bool) {
        if let Some(device) = &self.device {
            if paused { device.pause() } else { device.resume() }
        }
    }
}
