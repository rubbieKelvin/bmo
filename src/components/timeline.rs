use gpui::{ParentElement, Render, Styled, div, red};

pub struct TimeLine;

impl TimeLine {
    pub fn new() -> Self {
        return Self;
    }
}

impl Render for TimeLine {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        return div().bg(red()).child("H").flex_grow().h_16();
    }
}
