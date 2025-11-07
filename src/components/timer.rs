use std::time::{Duration, Instant};

use gpui::{Context, ParentElement, Render, Task, div};

use crate::session::Session;

pub struct Timer {
    pub is_running: bool,
    // the timer countdown, from the duration to 0
    pub countdown: Duration,
    pub timer_task: Option<Task<()>>,
}

impl Timer {
    pub fn new() -> Self {
        return Self {
            is_running: false,
            countdown: Duration::ZERO,
            timer_task: None,
        };
    }

    pub fn is_completed(&self) -> bool {
        return self.countdown.is_zero();
    }

    #[allow(unused)]
    pub fn is_paused(&self) -> bool {
        return !self.is_completed() && !self.is_running;
    }

    pub fn start(&mut self, session: &Session, cx: &mut Context<Timer>) {
        self.countdown = session.duration.clone();
        self.is_running = true;
        self.spawn_timer(cx);
    }

    pub fn pause(&mut self, cx: &mut Context<Timer>) {
        self.is_running = false;
        self.discard_timer();
        cx.notify();
    }

    pub fn play(&mut self, cx: &mut Context<Timer>) {
        self.is_running = true;
        self.spawn_timer(cx);
    }

    pub fn stop(&mut self, cx: &mut Context<Timer>) {
        self.is_running = false;
        self.countdown = Duration::ZERO;
        self.discard_timer();
        cx.notify();
    }

    fn discard_timer(&mut self) {
        if let Some(task) = self.timer_task.take() {
            drop(task);
        }
    }

    fn spawn_timer(&mut self, cx: &mut Context<Timer>) {
        self.timer_task = Some(cx.spawn(async |entity, cx| {
            let mut time_since_last_loop = Instant::now();
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;

                let time_elasped = Instant::now() - time_since_last_loop;
                time_since_last_loop = Instant::now();

                let should_continue_loop = entity.update(cx, |entity, cx| {
                    if !entity.is_running || entity.is_completed() {
                        cx.notify();
                        return false;
                    }

                    // tick
                    if time_elasped > entity.countdown {
                        entity.countdown = Duration::ZERO
                    } else {
                        entity.countdown -= time_elasped;
                    }

                    // at this point, if the countdown is zero
                    // we can call an event
                    if entity.countdown.is_zero() {
                        // TODO: event?
                    }

                    cx.notify();
                    return true;
                });

                if !should_continue_loop.unwrap_or(false) {
                    break;
                }
            }
            return ();
        }))
    }
}

impl Render for Timer {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        return div().child("00:00");
    }
}
