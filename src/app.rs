// use std::time::{Duration, Instant};

// use gpui::{
//     AppContext, Context, Div, Entity, InteractiveElement, IntoElement, MouseButton, MouseUpEvent,
//     ParentElement, Render, Styled, Task, Window, div, prelude::FluentBuilder, px, rgb, svg, white,
// };

// use crate::components::Segment;

use gpui::{AppContext, Context, Entity, ParentElement, Render, Styled, div, white};

use crate::components::timer::Timer;
use crate::session::TimerPreset;

pub struct BmoApp {
    timer: Entity<Timer>,
    session: TimerPreset,
}

impl BmoApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        return BmoApp {
            timer: cx.new(|_| Timer::new()),
            session: TimerPreset::default(),
        };
    }
}

impl Render for BmoApp {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        return div().bg(white()).child(self.timer.clone());
    }
}

// pub struct TimerApp {
//     remaining_seconds: u128,                  // count down
//     session_progress: u8,                     // how many sessions have we passed
//     current_session: Entity<PomorodoSession>, // active session
//     timer_task: Option<Task<()>>,             // active timer
//     is_running: bool,
//     is_paused: bool,
//     is_break: bool,
//     has_finished: bool,
// }

// impl TimerApp {
//     pub fn new(cx: &mut Context<Self>) -> Self {
//         return Self {
//             remaining_seconds: 0,
//             session_progress: 0,
//             current_session: cx.new(|_| PomorodoSession::default()),
//             timer_task: None,
//             is_paused: false,
//             is_running: false,
//             is_break: false,
//             has_finished: false,
//         };
//     }

//     // moves on to the next session
//     // returns if there was a new session to go to
//     fn roll_over(&mut self, cx: &mut Context<Self>) -> bool {
//         let current_session = self.current_session.read(cx);
//         if self.session_progress == current_session.session_count {
//             return false;
//         }

//         if !self.is_break {
//             self.session_progress += 1;
//         }

//         self.remaining_seconds = if self.is_break {
//             current_session.break_duration
//         } else {
//             current_session.focus_duration
//         };

//         cx.notify();
//         return true;
//     }

//     fn start_timer(&mut self, cx: &mut Context<Self>, fresh_start: bool) {
//         if self.is_running && fresh_start {
//             return;
//         }

//         // set initials
//         self.is_running = true;
//         self.has_finished = false;

//         if fresh_start {
//             self.is_break = false;
//             self.roll_over(cx);
//         }

//         // spawn timer task
//         self.timer_task = Some(cx.spawn(async move |entity, cx| {
//             let mut time_at_the_beginning = Instant::now();
//             loop {
//                 // wait 200 ms
//                 cx.background_executor()
//                     .timer(Duration::from_millis(200))
//                     .await;

//                 // get how much time has passed
//                 let time_passed = (Instant::now() - time_at_the_beginning).as_millis();
//                 time_at_the_beginning = Instant::now();

//                 // process
//                 let running = entity.update(cx, |entity, cx| {
//                     if !entity.is_running || entity.is_paused {
//                         cx.notify();
//                         return false;
//                     }

//                     if entity.remaining_seconds == 0 {
//                         entity.is_break = !entity.is_break;
//                         if !entity.roll_over(cx) {
//                             // reset internals
//                             entity.has_finished = true;
//                             entity.is_running = false;
//                             entity.is_paused = false;
//                             cx.notify();
//                             return false;
//                         };
//                     } else {
//                         if time_passed > entity.remaining_seconds {
//                             entity.remaining_seconds = 0;
//                         } else {
//                             entity.remaining_seconds -= time_passed;
//                         }
//                     }

//                     cx.notify();
//                     return true;
//                 });

//                 if !running.unwrap_or(false) {
//                     break;
//                 }
//             }
//         }));
//     }

//     fn pause_timer(&mut self, cx: &mut Context<Self>) {
//         self.is_paused = true;
//         cx.notify();
//     }

//     fn stop_timer(&mut self, cx: &mut Context<Self>) {
//         self.is_running = true;
//         cx.notify();
//     }

//     fn handle_action_button(
//         &mut self,
//         _event: &MouseUpEvent,
//         _window: &mut Window,
//         cx: &mut Context<Self>,
//     ) {
//         if !self.is_running {
//             self.start_timer(cx, true);
//         } else {
//             if self.is_paused {
//                 self.is_paused = false;
//                 self.start_timer(cx, false);
//             } else {
//                 self.pause_timer(cx);
//             }
//         }
//     }

//     fn handle_stop_button(
//         &mut self,
//         _event: &MouseUpEvent,
//         _window: &mut Window,
//         cx: &mut Context<Self>,
//     ) {
//         self.stop_timer(cx);
//     }

//     fn button(&self, fill: bool) -> Div {
//         return div()
//             .border(px(1.))
//             .border_color(rgb(0x5F5F5F))
//             .rounded_full()
//             .p_3()
//             .flex()
//             .flex_row()
//             .items_center()
//             .justify_center()
//             .when(fill, |el| el.bg(rgb(0x242424)))
//             .hover(|e| e.bg(rgb(0x141414)));
//     }

//     fn focus_sessions_indicator(&self, session_count: u8, currently_at: u8) -> Div {
//         return div()
//             .flex_row()
//             .flex()
//             .gap_2()
//             .children((1..session_count + 1).into_iter().map(|index| {
//                 div()
//                     .h(px(12.))
//                     .w(px(5.))
//                     .bg(rgb(if currently_at >= index {
//                         0xE93131
//                     } else {
//                         0x424242
//                     }))
//                     .rounded_full()
//             }));
//     }

//     fn timer_widget(&self, cx: &mut Context<Self>) -> Div {
//         return div()
//             .flex()
//             .flex_col()
//             .gap_4()
//             .justify_center()
//             .items_center()
//             // state icon
//             .child(
//                 svg()
//                     .when(!self.is_break, |el| el.path("svg/eye.svg"))
//                     .when(self.is_break, |el| el.path("svg/coffee.svg"))
//                     .size_8()
//                     .text_color(white()),
//             )
//             // timer count
//             .child(Segment::div(self.remaining_seconds, self.is_break).w_full())
//             // focus session count
//             .child(self.focus_sessions_indicator(
//                 self.current_session.read(cx).session_count,
//                 self.session_progress,
//             ))
//             // state text
//             .child(
//                 div()
//                     .child(if self.is_break { "BREAK" } else { "FOCUS" })
//                     .text_color(rgb(0x4F4F4F))
//                     .text_size(px(10.)),
//             )
//             .h(px(300.))
//             .w(px(300.))
//             .border_4()
//             .border_color(rgb(0x3A3A3A))
//             .rounded_full();
//     }

//     fn bottom_button_row(&self, cx: &mut Context<Self>) -> Div {
//         let action_text = if self.is_running {
//             if self.is_paused { "CONTINUE" } else { "PAUSE" }
//         } else {
//             "START"
//         };

//         return div()
//             .flex()
//             .flex_row()
//             .gap_4()
//             .when(self.is_running, |el| {
//                 el.child(
//                     self.button(false)
//                         .child(
//                             svg()
//                                 .path("svg/stop.svg")
//                                 .size_8()
//                                 .text_color(rgb(0x545454)),
//                         )
//                         .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_stop_button)),
//                 )
//             })
//             // Action button
//             .child(
//                 self.button(true)
//                     .text_color(white())
//                     .flex_grow()
//                     .text_center()
//                     .child(div().child(action_text))
//                     .when(!self.is_running, |el| el.items_start())
//                     .on_mouse_up(
//                         MouseButton::Left,
//                         cx.listener(TimerApp::handle_action_button),
//                     ),
//             )
//             .child(
//                 self.button(false).child(
//                     svg()
//                         .path("svg/settings.svg")
//                         .size_8()
//                         .text_color(rgb(0x545454)),
//                 ),
//             );
//     }
// }

// impl Render for TimerApp {
//     fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
//         return div()
//             .bg(rgb(0x090706))
//             .size_full()
//             .p_10()
//             .flex()
//             .flex_col()
//             .gap_4()
//             .justify_around()
//             .items_center()
//             .children([self.timer_widget(cx), self.bottom_button_row(cx).w_full()]);
//     }
// }
