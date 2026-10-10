use serde::{Deserialize, Serialize};

pub const START_HOUR: f64 = 6.0;
pub const MINUTES_PER_SECOND: f64 = 4.0;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimClock {
    pub elapsed: f64,
    pub scale: f32,
    pub paused: bool,
}

impl Default for SimClock {
    fn default() -> Self {
        SimClock {
            elapsed: 0.0,
            scale: 1.0,
            paused: false,
        }
    }
}

impl SimClock {
    pub fn sim_dt(&self, real_dt: f32) -> f32 {
        if self.paused {
            0.0
        } else {
            real_dt * self.scale.max(0.0)
        }
    }

    pub fn tick(&mut self, real_dt: f32) {
        self.elapsed += self.sim_dt(real_dt) as f64;
    }

    pub fn total_minutes(&self) -> f64 {
        START_HOUR * 60.0 + self.elapsed * MINUTES_PER_SECOND
    }

    pub fn day(&self) -> u32 {
        (self.total_minutes() / (24.0 * 60.0)).floor() as u32 + 1
    }

    pub fn clock(&self) -> (u32, u32) {
        let minutes = self.total_minutes().rem_euclid(24.0 * 60.0);
        let hours = (minutes / 60.0).floor() as u32;
        let mins = minutes.floor() as u32 % 60;
        (hours % 24, mins)
    }

    pub fn format_time(&self) -> String {
        let (h, m) = self.clock();
        format!("{h:02}:{m:02}")
    }

    pub fn format_day(&self) -> String {
        format!("День {}", self.day())
    }

    pub fn hour_fraction(&self) -> f32 {
        (self.total_minutes() / 60.0).rem_euclid(24.0) as f32
    }

    pub fn daylight(&self) -> f32 {
        const DAWN: f32 = 5.0;
        const DUSK: f32 = 22.0;
        let h = self.hour_fraction();
        let base = if h <= DAWN || h >= DUSK {
            0.0
        } else {
            (((h - DAWN) / (DUSK - DAWN) * std::f32::consts::PI).sin()).max(0.0)
        };
        0.15 + 0.85 * base.clamp(0.0, 1.0)
    }

    pub fn spawn_multiplier(&self) -> f32 {
        let h = self.hour_fraction();
        let peak = |center: f32, width: f32| (-(((h - center) / width).powi(2))).exp();
        let value = 0.22 + 2.4 * peak(8.0, 1.4) + 2.6 * peak(18.0, 1.6) + 0.5 * peak(13.0, 3.0);
        value.clamp(0.15, 3.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_six() {
        let c = SimClock::default();
        assert_eq!(c.format_time(), "06:00");
        assert_eq!(c.day(), 1);
    }

    #[test]
    fn pause_stops_time() {
        let mut c = SimClock {
            paused: true,
            ..Default::default()
        };
        c.tick(10.0);
        assert_eq!(c.elapsed, 0.0);
    }

    #[test]
    fn time_advances_by_minutes_per_second() {
        let mut c = SimClock {
            scale: 1.0,
            ..Default::default()
        };
        c.tick(30.0);
        assert_eq!(c.format_time(), "08:00");
    }

    #[test]
    fn daylight_is_brighter_at_noon() {
        let noon = SimClock {
            elapsed: 90.0,
            ..Default::default()
        };
        let morning = SimClock::default(); // 06:00
        assert!(noon.daylight() > morning.daylight());
        assert!(noon.daylight() > 0.5);
    }

    #[test]
    fn rush_hour_has_more_passengers() {
        let rush = SimClock {
            elapsed: 30.0, // 08:00
            ..Default::default()
        };
        let night = SimClock {
            elapsed: 300.0, // 02:00 следующего дня
            ..Default::default()
        };
        assert!(rush.spawn_multiplier() > night.spawn_multiplier() * 4.0);
    }

    #[test]
    fn a_day_passes_within_reasonable_time() {
        let mut c = SimClock {
            scale: 8.0,
            ..Default::default()
        };
        c.tick(45.0);
        assert_eq!(c.day(), 2);
    }
}
