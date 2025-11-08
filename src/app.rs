use gpui::prelude::FluentBuilder;
use gpui::{
    AppContext, Context, Div, Entity, InteractiveElement, ParentElement, Render, Styled, div, px,
    rgb, svg, white,
};
use gpui_component::ActiveTheme as _;
use gpui_component::TitleBar;

use crate::components::timeline::TimeLine;
use crate::components::timer::Timer;
use crate::session::{Session, SessionKind, TimerPreset};

pub struct BmoApp {
    timer: Entity<Timer>,
    timeline: Entity<TimeLine>,
    session_index: usize,
    preset: TimerPreset,
}

impl BmoApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        return BmoApp {
            timer: cx.new(|_| Timer::new()),
            timeline: cx.new(|_| TimeLine::new()),
            preset: TimerPreset::default(),
            session_index: 0,
        };
    }

    fn session(&self) -> &Session {
        let i = self.session_index;
        return self.preset.sessions.get(i).unwrap();
    }

    fn timer_area(&mut self) -> Div {
        let current_session = self.session();
        return div()
            .child(svg().size(px(32.)).text_color(white()).when_else(
                matches!(current_session.kind, SessionKind::WORK),
                |e| e.path("svg/eye.svg"),
                |e| e.path("svg/coffee.svg"),
            ))
            .child(self.timer.clone())
            .child(div().child(current_session.title.clone()))
            .flex()
            .gap_2()
            .flex_col()
            .flex_grow()
            .justify_center()
            .items_center();
    }

    fn button(&mut self, path: &str, cx: &mut Context<Self>) -> Div {
        return div()
            .size_16()
            .child(
                svg()
                    .size_7()
                    .text_color(cx.theme().foreground)
                    .path(path.to_string()),
            )
            .flex()
            .flex_row()
            .items_center()
            .justify_center();
    }

    fn running_footer_row(&mut self, cx: &mut Context<Self>) -> Div {
        return div()
            .flex()
            .flex_row()
            .gap_2()
            .items_center()
            .justify_around()
            .child(self.button("icons/pause.svg", cx))
            .child(self.timeline.clone())
            .child(self.button("icons/stop.svg", cx));
    }

    fn idle_footer(&mut self, cx: &mut Context<Self>) -> Div {
        return div()
            .p_4()
            .child(
                div()
                    .child("Start")
                    .flex_grow()
                    .text_center()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_full()
                    .py_4()
                    .hover(|el| el.bg(rgb(0x121212)))
                    .on_mouse_up(
                        gpui::MouseButton::Left,
                        cx.listener(|entity, _event, _win, cx| {
                            let session = entity.session();
                            entity.timer.update(cx, |entity, cx| {
                                entity.start(session, cx);
                            })
                        }),
                    ),
            )
            .flex()
            .flex_row()
            .items_center()
            .justify_center();
    }

    fn app_container(&mut self, cx: &mut Context<Self>) -> Div {
        let footer = self.running_footer_row(cx);
        let idle_footer = self.idle_footer(cx);

        return div()
            .flex_grow()
            .flex()
            .flex_col()
            .child(self.timer_area())
            .when_else(
                self.timer.read(cx).is_completed(),
                |el| el.child(idle_footer),
                |el| el.child(footer),
            );
    }
}

impl Render for BmoApp {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        let title = format!("Bmo ・ {}", self.preset.title.clone());
        return div()
            .size_full()
            .flex()
            .flex_col()
            .child(TitleBar::new().child(div().child(title)))
            .child(self.app_container(cx));
    }
}
