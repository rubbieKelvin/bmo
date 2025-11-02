// pomorodo session info
pub struct PomodoroSession {
    pub session_count: u8,
    pub break_duration: u32,
    pub focus_duration: u32,
}

impl Default for PomodoroSession {
    fn default() -> Self {
        return Self {
            session_count: 4,
            // break_duration: 60 * 10, // ten minutes
            // focus_duration: 60 * 60, // one hour
            break_duration: 60 * 1,
            focus_duration: 60 * 2,
        };
    }
}
