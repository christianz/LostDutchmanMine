//! The audio output: the game's AdLib and PC speaker, synthesised for a sound
//! device. It receives the port writes the simulation made and never feeds
//! anything back, so audio cannot change how the game runs.

use ymfm_sys::Chip;

/// Output samples per second.
pub const SAMPLE_RATE: u32 = 48_000;
/// The OPL2's input clock; it produces one sample every 72 clocks.
const OPL_CLOCK: f64 = 3_579_545.0;
const CLOCKS_PER_SAMPLE: u32 = 72;
/// Input clocks each port access costs, as in the simulation's chip.
const ACCESS_CLOCKS: u32 = 12;
/// The mix: FM, and the speaker's square wave beside it.
const FM_LEVEL: f32 = 0.7;
const SPEAKER_LEVEL: f32 = 0.08;

/// A second OPL2 fed with the simulation's writes, and the speaker's tone.
#[derive(Debug, Default)]
pub struct Synth {
    chip: Chip,
    /// How far between the chip's last two samples the output stands.
    fraction: f64,
    previous: f32,
    current: f32,
    /// The speaker's position in its cycle, from 0 to 1.
    phase: f64,
    speaker_hz: u32,
}

impl Synth {
    /// A silent synthesiser.
    pub fn new() -> Self {
        Synth::default()
    }

    /// An OPL port write: the address (offset 0) or data (offset 1) port.
    pub fn write(&mut self, offset: u16, value: u8) {
        self.chip.advance(ACCESS_CLOCKS);
        self.chip.write(offset, value);
    }

    /// The speaker's frequency, or zero for silence.
    pub fn speaker(&mut self, hz: u32) {
        self.speaker_hz = hz;
    }

    /// Fills `output` with mono samples at [`SAMPLE_RATE`]. The chip's own rate
    /// is resampled linearly.
    pub fn render(&mut self, output: &mut [f32]) {
        let chip_samples_per_output =
            OPL_CLOCK / f64::from(CLOCKS_PER_SAMPLE) / f64::from(SAMPLE_RATE);
        for sample in output {
            self.fraction += chip_samples_per_output;
            while self.fraction >= 1.0 {
                self.fraction -= 1.0;
                let generated = self.chip.generate();
                self.chip.advance(CLOCKS_PER_SAMPLE);
                self.previous = self.current;
                self.current = (f64::from(generated) / 32_768.0) as f32;
            }
            let fm = self.previous + (self.current - self.previous) * self.fraction as f32;
            self.phase += f64::from(self.speaker_hz) / f64::from(SAMPLE_RATE);
            self.phase -= self.phase.floor();
            let speaker = match self.speaker_hz {
                0 => 0.0,
                _ if self.phase < 0.5 => SPEAKER_LEVEL,
                _ => -SPEAKER_LEVEL,
            };
            *sample = (fm * FM_LEVEL + speaker).clamp(-1.0, 1.0);
        }
    }
}
