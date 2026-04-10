use crate::session::TimerPreset;

#[derive(Debug, Clone)]
pub enum Screen {
    Timer,
    Settings,
}

#[derive(Debug, Clone)]
pub struct NavigationEvent {
    pub screen: Screen,
    pub timer_preset: Option<TimerPreset>,
}
