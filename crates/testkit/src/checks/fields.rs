//! Report fields under the names the C++ capture JSON gave them, for checks
//! that compare several fields in turn and name the one that differs.

use game::Report;

/// One report field.
#[derive(Clone, Copy, Debug)]
pub struct Field {
    /// Its key in the capture JSON, which the verifiers' messages use.
    pub key: &'static str,
    /// Reads it; every field compared this way is unsigned.
    read: fn(&Report) -> u64,
}

impl Field {
    /// Its value in `report`.
    pub fn of(self, report: &Report) -> u64 {
        (self.read)(report)
    }
}

/// Translated steps run, which the C++ called boundaries.
pub const BOUNDARIES: Field = Field { key: "boundaries", read: |r| r.steps };
/// See [`Report::x`].
pub const X: Field = Field { key: "x", read: |r| u64::from(r.x) };
/// See [`Report::y`].
pub const Y: Field = Field { key: "y", read: |r| u64::from(r.y) };
/// See [`Report::town_page`].
pub const TOWN_PAGE: Field = Field { key: "town_page", read: |r| u64::from(r.town_page) };
/// See [`Report::building`].
pub const BUILDING: Field = Field { key: "building", read: |r| u64::from(r.building) };
/// See [`Report::map_scroll_x`].
pub const MAP_SCROLL_X: Field = Field { key: "map_scroll_x", read: |r| u64::from(r.map_scroll_x) };
/// See [`Report::map_scroll_y`].
pub const MAP_SCROLL_Y: Field = Field { key: "map_scroll_y", read: |r| u64::from(r.map_scroll_y) };
/// See [`Report::bullets`].
pub const BULLETS: Field = Field { key: "bullets", read: |r| u64::from(r.bullets) };
/// See [`Report::gold_bags`].
pub const GOLD_BAGS: Field = Field { key: "gold_bags", read: |r| u64::from(r.gold_bags) };
/// See [`Report::panning`].
pub const PANNING_ACTIVE: Field = Field { key: "panning_active", read: |r| u64::from(r.panning) };
/// See [`Report::mining_strokes`].
pub const MINING_STROKES: Field =
    Field { key: "mining_strokes", read: |r| u64::from(r.mining_strokes) };

/// Where the player stands, the verifiers' four-field `position`.
pub const POSITION: [Field; 4] = [X, Y, TOWN_PAGE, BUILDING];

/// The player's `(x, y)`; some verifiers call this `position`, others `point`.
pub fn point(report: &Report) -> (u16, u16) {
    (report.x, report.y)
}

/// The player's `(x, y, town_page, building)`.
pub fn position(report: &Report) -> (u16, u16, u16, u16) {
    (report.x, report.y, report.town_page, report.building)
}
