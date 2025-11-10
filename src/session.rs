// pomorodo session info
pub struct PomodoroSession {
    pub session_count: u8,
    pub break_duration: u128,
    pub focus_duration: u128,
}

impl Default for PomodoroSession {
    fn default() -> Self {
        return Self {
            session_count: 4,
            // break_duration: 60 * 10, // ten minutes
            // focus_duration: 60 * 60, // one hour
            break_duration: 60 * 1 * 1000,
            focus_duration: 60 * 2 * 1000,
        };
    }
}
