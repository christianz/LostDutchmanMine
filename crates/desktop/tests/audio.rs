//! The audio output: the game's OPL writes and speaker tone, synthesised.

use desktop::audio::{SAMPLE_RATE, Synth};

fn render(synth: &mut Synth, count: usize) -> Vec<f32> {
    let mut samples = vec![0.0; count];
    synth.render(&mut samples);
    samples
}

#[test]
fn a_silent_game_renders_silence() {
    let mut synth = Synth::new();
    assert!(render(&mut synth, 4800).iter().all(|&sample| sample == 0.0));
}

#[test]
fn the_speaker_is_a_square_wave_at_its_frequency() {
    let mut synth = Synth::new();
    synth.speaker(1000);
    let samples = render(&mut synth, SAMPLE_RATE as usize);
    let period = SAMPLE_RATE as usize / 1000;
    assert!(samples[..period / 2 - 2].iter().all(|&s| (s - 0.08).abs() < 1e-6), "high half");
    assert!(
        samples[period / 2 + 1..period - 2].iter().all(|&s| (s + 0.08).abs() < 1e-6),
        "low half"
    );
    let rises = samples.windows(2).filter(|pair| pair[0] < 0.0 && pair[1] > 0.0).count();
    assert!((999..=1000).contains(&rises), "a thousand cycles a second: {rises}");
    synth.speaker(0);
    assert!(render(&mut synth, 480).iter().all(|&sample| sample == 0.0));
}

#[test]
fn a_keyed_fm_note_sounds() {
    let mut synth = Synth::new();
    // Channel 0: both operators loud with fast envelopes, then key on at a
    // mid-range frequency.
    for (register, value) in [
        (0x20, 0x01),
        (0x23, 0x01),
        (0x40, 0x10),
        (0x43, 0x00),
        (0x60, 0xf0),
        (0x63, 0xf0),
        (0x80, 0x77),
        (0x83, 0x77),
        (0xa0, 0x98),
        (0xb0, 0x31),
    ] {
        synth.write(0, register);
        synth.write(1, value);
    }
    let samples = render(&mut synth, 4800);
    let loudest = samples.iter().fold(0.0f32, |max, &s| max.max(s.abs()));
    assert!(loudest > 0.01, "the note is audible: {loudest}");
    assert!(samples.iter().all(|&s| (-1.0..=1.0).contains(&s)));
}
