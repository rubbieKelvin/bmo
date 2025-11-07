use std::time::Duration;

use gpui::SharedString;

use crate::constants::ONE_MIN_MS;

// pomorodo session info
pub struct TimerPreset {
    pub title: SharedString,
    pub sessions: Vec<Session>,
}

impl TimerPreset {
    // the total duration
    pub fn total_duration(&self) -> Duration {
        return self
            .sessions
            .iter()
            .map(|i| i.duration)
            .reduce(|acc, e| acc + e)
            .unwrap_or_else(|| Duration::from_micros(0));
    }
}

impl Default for TimerPreset {
    fn default() -> Self {
        return Self {
            title: "Poromodo".into(),
            sessions: vec![
                Session::new("Focus".into(), Duration::from_millis(ONE_MIN_MS * 60)),
                Session::new("Short break".into(), Duration::from_millis(ONE_MIN_MS * 10)),
                Session::new("Focus".into(), Duration::from_millis(ONE_MIN_MS * 60)),
                Session::new("Long break".into(), Duration::from_millis(ONE_MIN_MS * 20)),
                Session::new("Focus".into(), Duration::from_millis(ONE_MIN_MS * 60)),
                Session::new("Short break".into(), Duration::from_millis(ONE_MIN_MS * 10)),
                Session::new("Focus".into(), Duration::from_millis(ONE_MIN_MS * 60)),
            ],
        };
    }
}

pub struct Session {
    pub title: SharedString,
    pub duration: Duration,
}

impl Session {
    pub fn new(title: SharedString, duration: Duration) -> Self {
        return Session { title, duration };
    }
}
