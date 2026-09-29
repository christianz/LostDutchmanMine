//! A summary of the game's state for scenario checks and the desktop's
//! screenshot reports, read between runs.

use machine::Machine;

use crate::symbols::{
    ASSAY_GRADE, ASSAY_POUNDS, BUILDING, BULLETS, CASH, GOLD_BAGS, MAP_SCROLL_X, MAP_SCROLL_Y,
    MINING_STROKES, MOUSE_MODE, POSITION_X, POSITION_Y, RETURN_X, RETURN_Y, SCENE_CAVE, SCENE_MAP,
    SURVIVAL_TICKS, TOWN_PAGE,
};
use crate::{Game, hooks, overlay};

/// What a scenario can check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools, reason = "a flat record of independent facts")]
pub struct Report {
    /// Translated steps run so far.
    pub steps: u64,
    /// The BIOS video mode.
    pub video_mode: u8,
    /// The player's position, or during an encounter the sight's corner.
    pub x: u16,
    /// See [`Report::x`].
    pub y: u16,
    /// The town page.
    pub town_page: u16,
    /// The building the player is in, or zero.
    pub building: u16,
    /// The directions the desktop holds.
    pub directions: u8,
    /// Whether the game set its own cursor shape.
    pub custom_cursor: bool,
    /// The mouse driver's visibility counter: shown from zero.
    pub mouse_visibility: i32,
    /// 1 while walking or aiming with the keyboard, 0 in hand-cursor mode.
    pub mouse_mode: u16,
    /// The pointer, in game pixels.
    pub mouse_x: i32,
    /// See [`Report::mouse_x`].
    pub mouse_y: i32,
    /// Whether a pointer is drawn.
    pub pointer_visible: bool,
    /// Whether a desert close-up waits for its dismissal.
    pub desert_view: bool,
    /// Whether the map is showing.
    pub map_view: bool,
    /// Whether a cave is showing.
    pub cave_view: bool,
    /// The map's scroll.
    pub map_scroll_x: u16,
    /// See [`Report::map_scroll_x`].
    pub map_scroll_y: u16,
    /// Where the player returns after a building, cave or river.
    pub return_x: u16,
    /// See [`Report::return_x`].
    pub return_y: u16,
    /// The survival clock's step counter.
    pub survival_ticks: u16,
    /// Whether the player is panning.
    pub panning: bool,
    /// Bags of gold carried.
    pub gold_bags: u16,
    /// Cash.
    pub cash: u32,
    /// The assay office's weight and grade of the bag being sold.
    pub assay_pounds: u16,
    /// See [`Report::assay_pounds`].
    pub assay_grade: u16,
    /// Whether the quality-of-life improvements are on.
    pub qol: bool,
    /// Whether an encounter is running.
    pub combat: bool,
    /// Bullets carried.
    pub bullets: u16,
    /// Where the sight shows.
    pub sight_x: i32,
    /// See [`Report::sight_x`].
    pub sight_y: i32,
    /// Whether Space is held for mining.
    pub mining_space_held: bool,
    /// Pick strokes in the current mining action.
    pub mining_strokes: u16,
}

impl Game {
    /// The state summary now.
    pub fn report(&self, m: &Machine) -> Report {
        let pointer = self.dos.mouse.input.current();
        let (sight_x, sight_y) = hooks::aim(m, self);
        Report {
            steps: m.steps,
            video_mode: self.dos.video.mode,
            x: POSITION_X.at_rest(m),
            y: POSITION_Y.at_rest(m),
            town_page: TOWN_PAGE.at_rest(m),
            building: BUILDING.at_rest(m),
            directions: self.held.directions,
            custom_cursor: self.dos.mouse.custom_cursor,
            mouse_visibility: self.dos.mouse.visibility,
            mouse_mode: MOUSE_MODE.at_rest(m),
            mouse_x: pointer.x,
            mouse_y: pointer.y,
            pointer_visible: overlay::pointer_visible(m, self),
            desert_view: self.desert_view,
            map_view: SCENE_MAP.at_rest(m) != 0,
            cave_view: SCENE_CAVE.at_rest(m) != 0,
            map_scroll_x: MAP_SCROLL_X.at_rest(m),
            map_scroll_y: MAP_SCROLL_Y.at_rest(m),
            return_x: RETURN_X.at_rest(m),
            return_y: RETURN_Y.at_rest(m),
            survival_ticks: SURVIVAL_TICKS.at_rest(m),
            panning: self.supplies.panning,
            gold_bags: GOLD_BAGS.at_rest(m),
            cash: u32::from(CASH.at_rest(m)) | u32::from(CASH.nth(1).at_rest(m)) << 16,
            assay_pounds: ASSAY_POUNDS.at_rest(m),
            assay_grade: ASSAY_GRADE.at_rest(m),
            qol: self.qol,
            combat: self.combat.active,
            bullets: BULLETS.at_rest(m),
            sight_x,
            sight_y,
            mining_space_held: self.supplies.mining_space_held,
            mining_strokes: MINING_STROKES.at_rest(m),
        }
    }
}
