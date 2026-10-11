//! Pure in-game clock value + scale helpers. Playback lives in [`crate::game_loop`].

/// Seconds in one in-game day (24 h).
pub const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

/// Real seconds → in-game seconds at speed ×1. 1 real second = 1 in-game minute.
pub const DEFAULT_TIME_SCALE: f64 = 60.0;

/// Discrete speed multipliers relative to [`DEFAULT_TIME_SCALE`].
pub const SPEED_MULTIPLIERS: [f64; 3] = [1.0, 10.0, 60.0];

/// Fixed sky anchors for the day/night path indicator (stage 1).
pub const SUNRISE_SECOND: u32 = 6 * 3600;
pub const NOON_SECOND: u32 = 12 * 3600;
pub const SUNSET_SECOND: u32 = 18 * 3600;

/// Calendar: 12 months × 30 days = 360-day year (stages 2–3).
pub const DAYS_PER_MONTH: u32 = 30;
pub const MONTHS_PER_YEAR: u32 = 12;
pub const DAYS_PER_YEAR: u32 = DAYS_PER_MONTH * MONTHS_PER_YEAR;
pub const MONTHS_PER_SEASON: u32 = 3;
pub const DAYS_PER_SEASON: u32 = DAYS_PER_MONTH * MONTHS_PER_SEASON;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameTime {
    /// 1-based absolute day index (continues across years).
    pub day: u32,
    /// Seconds elapsed within the current day `[0, SECONDS_PER_DAY)`.
    pub second_of_day: u32,
}

/// Celestial body shown on the sky-path indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkyBody {
    Sun,
    Moon,
}

/// Season of the year (3 months each; year starts in spring).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Spring => "Wiosna",
            Self::Summer => "Lato",
            Self::Autumn => "Jesień",
            Self::Winter => "Zima",
        }
    }
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

    /// 1-based year (day 1 → year 1).
    pub fn year(self) -> u32 {
        (self.day.saturating_sub(1)) / DAYS_PER_YEAR + 1
    }

    /// Day within the year in `1..=DAYS_PER_YEAR`.
    pub fn day_of_year(self) -> u32 {
        (self.day.saturating_sub(1)) % DAYS_PER_YEAR + 1
    }

    /// Month in `1..=MONTHS_PER_YEAR`.
    pub fn month(self) -> u32 {
        (self.day_of_year() - 1) / DAYS_PER_MONTH + 1
    }

    /// Day within the month in `1..=DAYS_PER_MONTH`.
    pub fn day_of_month(self) -> u32 {
        (self.day_of_year() - 1) % DAYS_PER_MONTH + 1
    }

    pub fn season(self) -> Season {
        match (self.month() - 1) / MONTHS_PER_SEASON {
            0 => Season::Spring,
            1 => Season::Summer,
            2 => Season::Autumn,
            _ => Season::Winter,
        }
    }

    pub fn format_label(self) -> String {
        let h = self.second_of_day / 3600;
        let m = (self.second_of_day % 3600) / 60;
        let s = self.second_of_day % 60;
        format!(
            "R{} · {:02}.{:02} · {} · {:02}:{:02}:{:02}",
            self.year(),
            self.day_of_month(),
            self.month(),
            self.season().label(),
            h,
            m,
            s
        )
    }

    /// Daytime is sunrise inclusive through sunset exclusive (`[06:00, 18:00)`).
    pub fn is_daytime(self) -> bool {
        self.second_of_day >= SUNRISE_SECOND && self.second_of_day < SUNSET_SECOND
    }

    pub fn sky_body(self) -> SkyBody {
        if self.is_daytime() {
            SkyBody::Sun
        } else {
            SkyBody::Moon
        }
    }

    /// Progress on the sky path: east `0.0` → south `0.5` → west `1.0`.
    /// Day tracks the sun (06→18); night tracks the moon (18→06).
    pub fn sky_path_progress(self) -> f32 {
        let day_len = (SUNSET_SECOND - SUNRISE_SECOND) as f32;
        if self.is_daytime() {
            (self.second_of_day - SUNRISE_SECOND) as f32 / day_len
        } else {
            let night_len = SECONDS_PER_DAY as f32 - day_len;
            let since_sunset = if self.second_of_day >= SUNSET_SECOND {
                self.second_of_day - SUNSET_SECOND
            } else {
                self.second_of_day + (SECONDS_PER_DAY as u32 - SUNSET_SECOND)
            };
            since_sunset as f32 / night_len
        }
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

    fn at_day(day: u32) -> GameTime {
        GameTime {
            day,
            second_of_day: 8 * 3600,
        }
    }

    #[test]
    fn format_start_morning() {
        let t = GameTime::start_of_day_one_at(8, 0, 0);
        assert_eq!(t.format_label(), "R1 · 01.01 · Wiosna · 08:00:00");
    }

    #[test]
    fn calendar_day_one() {
        let t = at_day(1);
        assert_eq!(t.year(), 1);
        assert_eq!(t.month(), 1);
        assert_eq!(t.day_of_month(), 1);
        assert_eq!(t.day_of_year(), 1);
        assert_eq!(t.season(), Season::Spring);
    }

    #[test]
    fn calendar_month_boundary() {
        let last_of_jan = at_day(DAYS_PER_MONTH);
        assert_eq!(last_of_jan.month(), 1);
        assert_eq!(last_of_jan.day_of_month(), 30);

        let first_of_feb = at_day(DAYS_PER_MONTH + 1);
        assert_eq!(first_of_feb.year(), 1);
        assert_eq!(first_of_feb.month(), 2);
        assert_eq!(first_of_feb.day_of_month(), 1);
        assert_eq!(first_of_feb.season(), Season::Spring);
    }

    #[test]
    fn calendar_year_boundary() {
        let last = at_day(DAYS_PER_YEAR);
        assert_eq!(last.year(), 1);
        assert_eq!(last.month(), 12);
        assert_eq!(last.day_of_month(), 30);
        assert_eq!(last.season(), Season::Winter);

        let next = at_day(DAYS_PER_YEAR + 1);
        assert_eq!(next.year(), 2);
        assert_eq!(next.month(), 1);
        assert_eq!(next.day_of_month(), 1);
        assert_eq!(next.season(), Season::Spring);
    }

    #[test]
    fn season_change_moments() {
        assert_eq!(at_day(1).season(), Season::Spring);
        assert_eq!(at_day(DAYS_PER_SEASON).season(), Season::Spring);
        assert_eq!(at_day(DAYS_PER_SEASON + 1).season(), Season::Summer);
        assert_eq!(at_day(DAYS_PER_SEASON * 2 + 1).season(), Season::Autumn);
        assert_eq!(at_day(DAYS_PER_SEASON * 3 + 1).season(), Season::Winter);
        assert_eq!(Season::Summer.label(), "Lato");
        assert_eq!(Season::Autumn.label(), "Jesień");
        assert_eq!(Season::Winter.label(), "Zima");
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
        assert_eq!(t.day_of_month(), 2);
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

    #[test]
    fn sky_path_day_anchors() {
        let sunrise = GameTime::start_of_day_one_at(6, 0, 0);
        assert!(sunrise.is_daytime());
        assert_eq!(sunrise.sky_body(), SkyBody::Sun);
        assert!((sunrise.sky_path_progress() - 0.0).abs() < 1e-6);

        let noon = GameTime {
            day: 1,
            second_of_day: NOON_SECOND,
        };
        assert_eq!(noon.sky_body(), SkyBody::Sun);
        assert!((noon.sky_path_progress() - 0.5).abs() < 1e-6);

        let almost_sunset = GameTime::start_of_day_one_at(17, 59, 59);
        assert!(almost_sunset.is_daytime());
        assert!(almost_sunset.sky_path_progress() > 0.99);
    }

    #[test]
    fn sky_path_night_anchors() {
        let sunset = GameTime {
            day: 1,
            second_of_day: SUNSET_SECOND,
        };
        assert!(!sunset.is_daytime());
        assert_eq!(sunset.sky_body(), SkyBody::Moon);
        assert!((sunset.sky_path_progress() - 0.0).abs() < 1e-6);

        let midnight = GameTime::start_of_day_one_at(0, 0, 0);
        assert_eq!(midnight.sky_body(), SkyBody::Moon);
        assert!((midnight.sky_path_progress() - 0.5).abs() < 1e-6);

        let almost_sunrise = GameTime::start_of_day_one_at(5, 59, 59);
        assert!(!almost_sunrise.is_daytime());
        assert!(almost_sunrise.sky_path_progress() > 0.99);
    }
}
