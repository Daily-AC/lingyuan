use serde::{Deserialize, Serialize};

pub const TICKS_PER_DAY: u32 = 72;
pub const DAYS_PER_SEASON: u32 = 10;
pub const SEASONS_PER_YEAR: u32 = 4;
pub const TICKS_PER_SEASON: u32 = TICKS_PER_DAY * DAYS_PER_SEASON;
pub const TICKS_PER_YEAR: u32 = TICKS_PER_SEASON * SEASONS_PER_YEAR;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Season {
    Chun,
    Xia,
    Qiu,
    Dong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DayPhase {
    Day,
    Dusk,
    Night,
    Dawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorldClock {
    pub tick: u64,
}

impl WorldClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance(&mut self) {
        self.tick += 1;
    }

    pub fn tick_in_day(&self) -> u32 {
        (self.tick as u32) % TICKS_PER_DAY
    }

    pub fn day_in_season(&self) -> u32 {
        (self.tick as u32 / TICKS_PER_DAY) % DAYS_PER_SEASON
    }

    pub fn season(&self) -> Season {
        let s = (self.tick as u32 / TICKS_PER_SEASON) % SEASONS_PER_YEAR;
        match s {
            0 => Season::Chun,
            1 => Season::Xia,
            2 => Season::Qiu,
            _ => Season::Dong,
        }
    }

    pub fn year(&self) -> u32 {
        self.tick as u32 / TICKS_PER_YEAR
    }

    pub fn phase(&self) -> DayPhase {
        match self.tick_in_day() {
            0..=29 => DayPhase::Day,
            30..=35 => DayPhase::Dusk,
            36..=65 => DayPhase::Night,
            _ => DayPhase::Dawn,
        }
    }

    pub fn is_night(&self) -> bool {
        matches!(self.phase(), DayPhase::Night)
    }

    pub fn just_changed_season(&self) -> bool {
        self.tick > 0 && (self.tick as u32) % TICKS_PER_SEASON == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_day_then_night() {
        let mut c = WorldClock::new();
        assert_eq!(c.phase(), DayPhase::Day);
        for _ in 0..36 {
            c.advance();
        }
        assert_eq!(c.phase(), DayPhase::Night);
    }

    #[test]
    fn season_cycle() {
        let mut c = WorldClock::new();
        assert_eq!(c.season(), Season::Chun);
        for _ in 0..TICKS_PER_SEASON {
            c.advance();
        }
        assert_eq!(c.season(), Season::Xia);
        assert!(c.just_changed_season());
    }

    #[test]
    fn year_advances() {
        let mut c = WorldClock::new();
        for _ in 0..TICKS_PER_YEAR {
            c.advance();
        }
        assert_eq!(c.year(), 1);
    }
}
