use std::time::Duration;

use gpui::{Context, Div, relative, rgba};
use gpui::{ParentElement, Render, SharedString, Styled, div, prelude::FluentBuilder, rems, rgb};
use gpui_component::ActiveTheme;

use crate::db::session_color_for;
use crate::session::TimerPreset;

pub struct TimeLineSegment {
    title: SharedString,
    color: u32,
}

pub struct TimeLine {
    pub active_index: usize,
    pub current_progress: f32, // current progress in percentage 0 - 1
    pub total_duration: Duration,
    pub segments: Vec<TimeLineSegment>,
}

impl TimeLine {
    pub fn new() -> Self {
        return Self {
            current_progress: 0.,
            active_index: 0,
            total_duration: Duration::ZERO,
            segments: vec![],
        };
    }

    pub fn update_segments(&mut self, preset: &TimerPreset) {
        self.total_duration = preset.total_duration();
        self.segments = preset
            .sessions
            .iter()
            .enumerate()
            .map(|(index, session)| TimeLineSegment {
                title: session.title.clone(),
                color: session_color_for(session.color, index),
            })
            .collect();
    }
}

fn segment_component(
    segment: &TimeLineSegment,
    active: bool,
    percent: f32,
    cx: &mut Context<TimeLine>,
) -> Div {
    return div()
        .min_w(rems(2.))
        .when_else(
            active,
            |e| {
                e.flex_grow()
                    .relative()
                    .child(div().w(relative(percent)).h_full().bg(rgba(0x00000055)))
                    .child(
                        div()
                            .absolute()
                            .flex()
                            .size_full()
                            .flex_row()
                            .items_center()
                            .justify_center()
                            .text_sm()
                            .child(segment.title.clone().to_uppercase())
                            .text_color(cx.theme().background),
                    )
                    .flex()
                    .items_center()
                    .h(rems(2.8))
                    .bg(rgb(segment.color))
            },
            |e| e.h(rems(1.6)).border_1().border_color(cx.theme().border),
        )
        .rounded_lg();
}

impl Render for TimeLine {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let segments = &self.segments;

        return div()
            .flex_grow()
            .h_16()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_xl()
            .relative()
            .child(
                div()
                    .absolute()
                    .px_2()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .size_full()
                    .children(segments.iter().enumerate().map(|(index, seg)| {
                        segment_component(
                            seg,
                            index == self.active_index,
                            self.current_progress,
                            cx,
                        )
                    })),
            );
    }
}
