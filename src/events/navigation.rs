use crate::db::Preset;

#[derive(Debug, Clone)]
pub enum Screen {
    Timer,
    Settings(Vec<Preset>),
    // PresetEdit,
}

#[derive(Debug, Clone)]
pub struct NavigationEvent {
    pub screen: Screen,
}
