//! Date, time and the BIOS tick count, all derived from emulated time.

use machine::Machine;

use crate::Dos;

/// 2026-01-01 12:00:00, the date deterministic sessions start on.
const DEFAULT_BASE: i64 = 1_767_268_800;

/// The session's clock. DOS date and time are `base` plus emulated time, never
/// the host clock, so a run reproduces exactly; a live session sets `base` to
/// local time when it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    /// Local time at the start of the session, in seconds since 1970-01-01.
    pub base: i64,
    /// Emulated milliseconds since the session started.
    pub emulated_ms: u64,
    /// The BIOS tick count at 0040:006C.
    pub ticks: u32,
}

impl Default for Clock {
    fn default() -> Self {
        Clock { base: DEFAULT_BASE, emulated_ms: 0, ticks: 0 }
    }
}

/// A calendar date.
struct Date {
    year: i64,
    month: u32,
    day: u32,
    weekday: u32,
}

/// Howard Hinnant's civil-from-days: exact for every proleptic Gregorian date.
fn civil_from_days(days: i64) -> Date {
    let weekday = (days % 7 + 11) % 7;
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let doe = days - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    Date {
        year: yoe + era * 400 + i64::from(month <= 2),
        month: month as u32,
        day: day as u32,
        weekday: weekday as u32,
    }
}

impl Clock {
    fn seconds(&self) -> i64 {
        self.base + (self.emulated_ms / 1000) as i64
    }

    /// INT 21h AH=2Ah: CX year, DH month, DL day, AL weekday (0 = Sunday).
    pub fn date(&self, m: &mut Machine) {
        let date = civil_from_days(self.seconds().div_euclid(86_400));
        m.regs.cx = date.year as u16;
        m.regs.dx = ((date.month << 8) | date.day) as u16;
        m.regs.set_al(date.weekday as u8);
    }

    /// INT 21h AH=2Ch: CH hour, CL minute, DH second, DL hundredths.
    pub fn time(&self, m: &mut Machine) {
        let time = self.seconds().rem_euclid(86_400);
        m.regs.cx = (((time / 3600) << 8) | (time / 60 % 60)) as u16;
        m.regs.dx = (((time % 60) << 8) as u64 | (self.emulated_ms % 1000 / 10)) as u16;
    }

    /// The BIOS timer handler: one tick, mirrored into the BIOS data area.
    pub fn tick(&mut self, m: &mut Machine) {
        self.ticks = self.ticks.wrapping_add(1);
        m.memory.write16(0x40, 0x6c, self.ticks as u16);
        m.memory.write16(0x40, 0x6e, (self.ticks >> 16) as u16);
    }
}

impl Dos {
    /// INT 1Ah AH=00h: the tick count in CX:DX.
    pub(crate) fn timer_service(&mut self, m: &mut Machine) -> bool {
        if m.regs.ah() != 0 {
            return false;
        }
        m.regs.cx = (self.clock.ticks >> 16) as u16;
        m.regs.dx = self.clock.ticks as u16;
        m.regs.set_al(0);
        true
    }
}
