use std::time::Duration;

use gpui::SharedString;

use crate::constants::ONE_MIN_MS;

// pomorodo session info
pub struct TimerPreset {
    pub title: SharedString,
    pub sessions: Vec<Session>,
}

impl TimerPreset {
    #[allow(unused)]
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
                Session::new(
                    "Focus".into(),
                    Duration::from_millis(ONE_MIN_MS * 60),
                    SessionKind::WORK,
                ),
                Session::new(
                    "Short break".into(),
                    Duration::from_millis(ONE_MIN_MS * 10),
                    SessionKind::BREAK,
                ),
                Session::new(
                    "Focus".into(),
                    Duration::from_millis(ONE_MIN_MS * 60),
                    SessionKind::WORK,
                ),
                Session::new(
                    "Long break".into(),
                    Duration::from_millis(ONE_MIN_MS * 20),
                    SessionKind::BREAK,
                ),
                Session::new(
                    "Focus".into(),
                    Duration::from_millis(ONE_MIN_MS * 60),
                    SessionKind::WORK,
                ),
                Session::new(
                    "Short break".into(),
                    Duration::from_millis(ONE_MIN_MS * 10),
                    SessionKind::BREAK,
                ),
                Session::new(
                    "Focus".into(),
                    Duration::from_millis(ONE_MIN_MS * 60),
                    SessionKind::WORK,
                ),
            ],
        };
    }
}

pub enum SessionKind {
    WORK,
    BREAK,
}

pub struct Session {
    pub title: SharedString,
    pub duration: Duration,
    pub kind: SessionKind,
}

impl Session {
    pub fn new(title: SharedString, duration: Duration, kind: SessionKind) -> Self {
        return Session {
            title,
            duration,
            kind,
        };
    }
}
