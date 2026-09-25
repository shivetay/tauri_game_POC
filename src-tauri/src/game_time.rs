//! Pure in-game clock value + scale helpers. Playback lives in [`crate::game_loop`].

/// Seconds in one in-game day (24 h).
pub const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

/// Real seconds → in-game seconds at speed ×1. 1 real second = 1 in-game minute.
pub const DEFAULT_TIME_SCALE: f64 = 60.0;

/// Discrete speed multipliers relative to [`DEFAULT_TIME_SCALE`].
pub const SPEED_MULTIPLIERS: [f64; 3] = [1.0, 10.0, 60.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameTime {
    /// 1-based day index.
    pub day: u32,
    /// Seconds elapsed within the current day `[0, SECONDS_PER_DAY)`.
    pub second_of_day: u32,
}

impl GameTime {
    pub const fn start_of_day_one_at(hour: u32, minute: u32, second: u32) -> Self {
        let sod = hour * 3600 + minute * 60 + second;
        Self {
            day: 1,
            second_of_day: sod,
        }
    }

    pub fn advance(&mut self, game_seconds: f64) {
        if !game_seconds.is_finite() || game_seconds <= 0.0 {
            return;
        }
        // Cap one tick so a long hitch does not jump many days.
        let add = game_seconds.min(SECONDS_PER_DAY as f64) as u64;
        if add == 0 {
            return;
        }
        let total = self.second_of_day as u64 + add;
        let days = (total / SECONDS_PER_DAY) as u32;
        self.second_of_day = (total % SECONDS_PER_DAY) as u32;
        self.day = self.day.saturating_add(days);
    }

    pub fn advance_hours(&mut self, hours: u32) {
        self.advance(hours as f64 * 3600.0);
    }

    pub fn format_label(self) -> String {
        let h = self.second_of_day / 3600;
        let m = (self.second_of_day % 3600) / 60;
        let s = self.second_of_day % 60;
        format!("Dzień {} · {:02}:{:02}:{:02}", self.day, h, m, s)
    }
}

/// Effective real→game scale from pause + speed multiplier.
pub fn effective_time_scale(paused: bool, speed_mult: f64) -> f64 {
    if paused || !speed_mult.is_finite() || speed_mult <= 0.0 {
        0.0
    } else {
        DEFAULT_TIME_SCALE * speed_mult
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_start_morning() {
        let t = GameTime::start_of_day_one_at(8, 0, 0);
        assert_eq!(t.format_label(), "Dzień 1 · 08:00:00");
    }

    #[test]
    fn advance_wraps_day() {
        let mut t = GameTime {
            day: 1,
            second_of_day: SECONDS_PER_DAY as u32 - 2,
        };
        t.advance(5.0);
        assert_eq!(t.day, 2);
        assert_eq!(t.second_of_day, 3);
    }

    #[test]
    fn advance_ignores_non_positive() {
        let mut t = GameTime::start_of_day_one_at(8, 0, 0);
        t.advance(0.0);
        t.advance(-1.0);
        assert_eq!(t, GameTime::start_of_day_one_at(8, 0, 0));
    }

    #[test]
    fn advance_hours_crosses_midnight() {
        let mut t = GameTime::start_of_day_one_at(23, 0, 0);
        t.advance_hours(2);
        assert_eq!(t.day, 2);
        assert_eq!(t.second_of_day, 3600);
    }

    #[test]
    fn effective_scale_respects_pause() {
        assert_eq!(effective_time_scale(true, 10.0), 0.0);
        assert_eq!(effective_time_scale(false, 1.0), DEFAULT_TIME_SCALE);
        assert_eq!(effective_time_scale(false, 60.0), DEFAULT_TIME_SCALE * 60.0);
    }
}
