use gpui::{
    Context, Div, InteractiveElement, IntoElement, ParentElement, Render, Styled, Window, div,
    prelude::FluentBuilder, px, rgb, white,
};

pub struct TimerApp {}

impl TimerApp {
    pub fn new() -> Self {
        return Self {};
    }

    fn button(&self, fill: bool) -> Div {
        return div()
            .border(px(1.))
            .border_color(rgb(0x5F5F5F))
            .rounded_full()
            .p_4()
            .when(fill, |el| el.bg(rgb(0x5F5F5F)))
            .hover(|e| e.bg(rgb(0x4B4B4B)));
    }

    fn timer_widget(&self) -> Div {
        return div().child("timer").text_color(white()).flex_grow();
    }

    fn button_row(&self) -> Div {
        return div()
            .flex()
            .flex_row()
            .gap_4()
            .child(self.button(false).child("return").text_color(white()))
            .child(
                self.button(true)
                    .child("START")
                    .text_color(white())
                    .flex_grow()
                    .text_center(),
            )
            .child(self.button(false).child("return").text_color(white()));
    }
}

impl Render for TimerApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        return div()
            .bg(rgb(0x090706))
            .size_full()
            .p_10()
            .flex()
            .flex_col()
            .gap_4()
            .children([self.timer_widget(), self.button_row()]);
    }
}
