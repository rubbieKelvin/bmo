use crate::session::TimerPreset;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Timer,
    Settings,
    /// Detailed preset/session editor for the given preset id.
    PresetEditor(i64),
}

#[derive(Debug, Clone)]
pub struct NavigationEvent {
    pub screen: Screen,
    pub timer_preset: Option<TimerPreset>,
}

impl NavigationEvent {
    pub fn goto(screen: Screen) -> Self {
        Self {
            screen,
            timer_preset: None,
        }
    }
}
