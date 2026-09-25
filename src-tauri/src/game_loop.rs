//! Simulation loop shell: advance world state from real `dt`.
//! UI reads this state and only mutates controls (pause / speed / skip).

use crate::game_time::{
    effective_time_scale, GameTime, SPEED_MULTIPLIERS,
};

/// Owns in-game time and playback controls. Call [`GameLoop::tick`] each frame.
#[derive(Debug, Clone)]
pub struct GameLoop {
    time: GameTime,
    paused: bool,
    speed_mult: f64,
}

impl Default for GameLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl GameLoop {
    pub fn new() -> Self {
        Self {
            time: GameTime::start_of_day_one_at(8, 0, 0),
            paused: false,
            speed_mult: SPEED_MULTIPLIERS[0],
        }
    }

    /// Advance simulation by real-time `dt` seconds (respects pause + speed).
    pub fn tick(&mut self, dt: f64) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let scale = effective_time_scale(self.paused, self.speed_mult);
        if scale > 0.0 {
            self.time.advance(dt * scale);
        }
    }

    pub fn time(&self) -> GameTime {
        self.time
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn speed_mult(&self) -> f64 {
        self.speed_mult
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn set_speed(&mut self, mult: f64) {
        self.speed_mult = mult;
        self.paused = false;
    }

    pub fn skip_hours(&mut self, hours: u32) {
        self.time.advance_hours(hours);
    }

    /// Restart the clock at day 1 · 08:00 (keeps pause / speed).
    pub fn reset_time(&mut self) {
        self.time = GameTime::start_of_day_one_at(8, 0, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_time::DEFAULT_TIME_SCALE;

    #[test]
    fn tick_advances_when_running() {
        let mut loop_ = GameLoop::new();
        let before = loop_.time().second_of_day;
        loop_.tick(1.0);
        assert_eq!(
            loop_.time().second_of_day,
            before + DEFAULT_TIME_SCALE as u32
        );
    }

    #[test]
    fn tick_noop_when_paused() {
        let mut loop_ = GameLoop::new();
        loop_.toggle_pause();
        let before = loop_.time();
        loop_.tick(1.0);
        assert_eq!(loop_.time(), before);
    }

    #[test]
    fn set_speed_unpauses() {
        let mut loop_ = GameLoop::new();
        loop_.toggle_pause();
        loop_.set_speed(10.0);
        assert!(!loop_.paused());
        assert_eq!(loop_.speed_mult(), 10.0);
    }

    #[test]
    fn skip_hours_bypasses_pause() {
        let mut loop_ = GameLoop::new();
        loop_.toggle_pause();
        loop_.skip_hours(1);
        assert_eq!(loop_.time().second_of_day, 8 * 3600 + 3600);
    }

    #[test]
    fn reset_time_returns_to_day_one() {
        let mut loop_ = GameLoop::new();
        loop_.skip_hours(30);
        assert!(loop_.time().day > 1);
        loop_.reset_time();
        assert_eq!(loop_.time(), GameTime::start_of_day_one_at(8, 0, 0));
    }
}
