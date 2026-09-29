//! Replays: a session's input with the millisecond it arrived before, as text.
//!
//! A replay and a starting point reproduce a session exactly, crash included.
//! It holds input only, no game content. Its header says what it needs to be
//! replayed faithfully: the build, the clock base, the settings.

use std::fmt::Write as _;

use crate::InputKind;

/// A recorded session.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Replay {
    /// Facts about the session, such as `build` and `clock-base`.
    pub header: Vec<(String, String)>,
    /// Every input, with the quantum it arrived before.
    pub inputs: Vec<(u64, InputKind)>,
}

/// A replay line that cannot be read.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("line {line}: {message}")]
pub struct ReplayError {
    /// The line, from 1.
    pub line: usize,
    /// What is wrong with it.
    pub message: String,
}

impl Replay {
    /// The replay as text: `# key value` header lines, then one
    /// `<ms> <kind> [values]` line per input.
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        for (key, value) in &self.header {
            let _ = writeln!(text, "# {key} {value}");
        }
        for (ms, input) in &self.inputs {
            let _ = writeln!(text, "{ms} {}", encode(*input));
        }
        text
    }

    /// Reads [`Replay::to_text`]'s format.
    ///
    /// # Errors
    ///
    /// [`ReplayError`] naming the first line that cannot be read.
    pub fn parse(text: &str) -> Result<Self, ReplayError> {
        let mut replay = Replay::default();
        for (index, line) in text.lines().enumerate() {
            let error = |message: String| ReplayError { line: index + 1, message };
            if let Some(header) = line.strip_prefix("# ") {
                let (key, value) = header.split_once(' ').unwrap_or((header, ""));
                replay.header.push((key.to_owned(), value.to_owned()));
            } else if !line.is_empty() {
                replay.inputs.push(decode(line).map_err(error)?);
            }
        }
        Ok(replay)
    }
}

fn encode(input: InputKind) -> String {
    match input {
        InputKind::Key(key) => format!("key {key}"),
        InputKind::Release(source) => format!("release {source}"),
        InputKind::Directions(directions) => format!("directions {directions}"),
        InputKind::Space(held) => format!("space {}", u8::from(held)),
        InputKind::Mouse { x, y } => format!("mouse {x} {y}"),
        InputKind::Buttons(buttons) => format!("buttons {buttons}"),
        InputKind::Clear => "clear".to_owned(),
        InputKind::Qol(on) => format!("qol {}", u8::from(on)),
    }
}

fn decode(line: &str) -> Result<(u64, InputKind), String> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let [ms, kind, values @ ..] = fields.as_slice() else {
        return Err("expected a time and an input".to_owned());
    };
    let ms = ms.parse().map_err(|_| format!("not a time: {ms}"))?;
    let value = |i: usize| -> Result<i64, String> {
        let field = values.get(i).ok_or_else(|| format!("{kind} needs {} values", i + 1))?;
        field.parse().map_err(|_| format!("not a number: {field}"))
    };
    let key =
        |i: usize| value(i).and_then(|v| u32::try_from(v).map_err(|_| format!("not a key: {v}")));
    let input = match *kind {
        "key" => InputKind::Key(key(0)?),
        "release" => InputKind::Release(key(0)?),
        "directions" => InputKind::Directions(value(0)? as u8),
        "space" => InputKind::Space(value(0)? != 0),
        "mouse" => InputKind::Mouse { x: value(0)? as i32, y: value(1)? as i32 },
        "buttons" => InputKind::Buttons(value(0)? as u8),
        "clear" => InputKind::Clear,
        "qol" => InputKind::Qol(value(0)? != 0),
        other => return Err(format!("unknown input: {other}")),
    };
    Ok((ms, input))
}
