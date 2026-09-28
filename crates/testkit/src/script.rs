//! Scenario scripts: desktop input with the emulated millisecond it arrives at.
//!
//! One event per line: `<ms> <type> [a] [b]`, missing arguments zero, `#`
//! lines ignored. Keys are SDL scancodes with SDL modifier bits, so a script
//! drives the same controller a window does.

/// What a script line does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// A BIOS key straight to the game.
    Key(u16),
    /// A physical key goes down.
    Down {
        /// SDL scancode.
        scancode: u16,
        /// SDL modifier bits.
        mods: u16,
    },
    /// A physical key goes up.
    Up {
        /// SDL scancode.
        scancode: u16,
        /// SDL modifier bits.
        mods: u16,
    },
    /// A held physical key repeats.
    Repeat {
        /// SDL scancode.
        scancode: u16,
        /// SDL modifier bits.
        mods: u16,
    },
    /// The window loses focus.
    FocusLost,
    /// A typed character, as a layout without a physical key would give it.
    Ascii(u8),
    /// The pointer moves, in game pixels.
    Mouse {
        /// Horizontal position.
        x: i32,
        /// Vertical position.
        y: i32,
    },
    /// The held mouse buttons change.
    Buttons(u8),
    /// A capture of the presented picture.
    Screen(u32),
    /// A capture of the game's frame and state.
    Capture(u32),
}

/// A timed action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    /// The emulated millisecond it happens at.
    pub at_ms: u64,
    /// What happens.
    pub action: Action,
}

/// A script line that cannot be run.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("line {line}: {message}")]
pub struct ScriptError {
    /// The line, from 1.
    pub line: usize,
    /// What is wrong with it.
    pub message: String,
}

/// One more than the highest SDL scancode.
const SCANCODES: i64 = 512;

/// Parses a script.
///
/// # Errors
///
/// [`ScriptError`] for a malformed line, an unknown event, an impossible
/// scancode, or a time before the previous line's.
pub fn parse(text: &str) -> Result<Vec<Event>, ScriptError> {
    let mut events: Vec<Event> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let error = |message: String| ScriptError { line: index + 1, message };
        let event = event(line).map_err(error)?;
        if events.last().is_some_and(|last| last.at_ms > event.at_ms) {
            return Err(error("the script is not in time order".to_owned()));
        }
        events.push(event);
    }
    Ok(events)
}

fn event(line: &str) -> Result<Event, String> {
    let mut fields = line.split_whitespace();
    let number = |field: Option<&str>| -> Result<i64, String> {
        field.map_or(Ok(0), |text| text.parse().map_err(|_| format!("not a number: {text}")))
    };
    let at_ms = number(fields.next())?;
    let kind = fields.next().ok_or("no event type")?;
    let (a, b) = (number(fields.next())?, number(fields.next())?);
    let narrow = |value: i64| u16::try_from(value).map_err(|_| format!("out of range: {value}"));
    let key = || {
        if (1..SCANCODES).contains(&a) {
            Ok((a as u16, narrow(b)?))
        } else {
            Err(format!("no such scancode: {a}"))
        }
    };
    let action = match kind {
        // The C++ desktop passed these on as 16 bits.
        "key" => Action::Key(a as u16),
        "down" => key().map(|(scancode, mods)| Action::Down { scancode, mods })?,
        "up" => key().map(|(scancode, mods)| Action::Up { scancode, mods })?,
        "repeat" => key().map(|(scancode, mods)| Action::Repeat { scancode, mods })?,
        "focuslost" => Action::FocusLost,
        "ascii" => Action::Ascii(u8::try_from(a).map_err(|_| format!("not a character: {a}"))?),
        "mouse" => Action::Mouse { x: a as i32, y: b as i32 },
        "buttons" => Action::Buttons(a as u8),
        "screen" => Action::Screen(a as u32),
        "capture" => Action::Capture(a as u32),
        other => return Err(format!("unknown event: {other}")),
    };
    let at_ms = u64::try_from(at_ms).map_err(|_| format!("a negative time: {at_ms}"))?;
    Ok(Event { at_ms, action })
}
