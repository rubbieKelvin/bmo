use gpui::prelude::FluentBuilder;
use gpui::{
    AppContext, Context, Div, Entity, EventEmitter, InteractiveElement, ParentElement, Render,
    Styled, div, px, rgb, svg, white,
};
use gpui_component::TitleBar;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{ActiveTheme as _, IconName};

use crate::audio::AudioPlayer;
use crate::components::timeline::TimeLine;
use crate::components::timer::{Timer, TimerCompletedEvent, TimerTickEvent};
use crate::events::navigation::{NavigationEvent, Screen};
use crate::notifications;
use crate::session::{Session, SessionKind, TimerPreset};

/// Fired when the final segment of the active preset finishes.
#[allow(dead_code)]
pub struct PresetCompletedEvent;

pub struct TimerScreen {
    timer: Entity<Timer>,
    timeline: Entity<TimeLine>,
    session_index: usize,
    preset: TimerPreset,
    auto_advance: bool,
    notifications_enabled: bool,
    sounds_enabled: bool,
}

impl EventEmitter<NavigationEvent> for TimerScreen {}
impl EventEmitter<PresetCompletedEvent> for TimerScreen {}

impl TimerScreen {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let timer = cx.new(|_| Timer::new());
        let timeline = cx.new(|_| TimeLine::new());

        // update the timeline on every tick
        cx.subscribe(&timer, {
            let timeline = timeline.clone();
            move |_parent, _timer, event: &TimerTickEvent, cx| {
                let percentage_completed = event.percent_completed;
                timeline.update(cx, |timeline, _cx| {
                    timeline.current_progress = percentage_completed;
                });
            }
        })
        .detach();

        // subscription to check for timer completed event
        cx.subscribe(&timer, {
            let timeline = timeline.clone();
            move |parent, timer, _event: &TimerCompletedEvent, cx| {
                parent.handle_segment_complete(&timer, &timeline, cx);
            }
        })
        .detach();

        let preset = TimerPreset::default();
        // Populate timeline segments so the idle screen is not empty.
        timeline.update(cx, |tl, _cx| tl.update_segments(&preset));

        return TimerScreen {
            timer,
            timeline,
            preset,
            session_index: 0,
            auto_advance: true,
            notifications_enabled: true,
            sounds_enabled: true,
        };
    }

    fn handle_segment_complete(
        &mut self,
        timer: &Entity<Timer>,
        timeline: &Entity<TimeLine>,
        cx: &mut Context<Self>,
    ) {
        let completed_session = self.session().clone();
        let is_last = self.session_index + 1 >= self.preset.sessions.len();

        // side-effects (notifications + sounds)
        if self.notifications_enabled {
            let body = if is_last {
                format!("Preset \"{}\" complete.", self.preset.title)
            } else {
                let next = self.preset.sessions.get(self.session_index + 1);
                match next {
                    Some(n) => format!(
                        "\"{}\" finished. Next: {}.",
                        completed_session.title, n.title
                    ),
                    None => format!("\"{}\" finished.", completed_session.title),
                }
            };
            notifications::notify_segment(&completed_session.title, &body);
        }
        if self.sounds_enabled {
            if is_last {
                AudioPlayer::play_complete();
            } else {
                AudioPlayer::play_ding();
            }
        }

        if is_last {
            // Reset to idle beginning-of-preset state.
            self.session_index = 0;
            timer.update(cx, |t, cx| t.stop(cx));
            timeline.update(cx, |tl, _| {
                tl.active_index = 0;
                tl.current_progress = 0.;
            });
            cx.emit(PresetCompletedEvent);
            cx.notify();
            return;
        }

        // advance to next segment
        let new_sess_index = self.session_index + 1;
        self.session_index = new_sess_index;
        timeline.update(cx, |e, _cx| {
            e.active_index = new_sess_index;
            e.current_progress = 0.;
        });

        if self.auto_advance {
            let session = self.session().clone();
            timer.update(cx, |e, cx| {
                e.start(&session, cx);
            });
        } else {
            // Leave the timer in its completed state so the idle "Start"
            // footer is shown. The user explicitly starts the next segment.
            timer.update(cx, |t, cx| t.stop(cx));
            cx.notify();
        }
    }

    fn session(&self) -> &Session {
        let i = self.session_index.min(self.preset.sessions.len().saturating_sub(1));
        return self.preset.sessions.get(i).expect("preset has at least one session");
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
            .border_1()
            .border_color(cx.theme().border)
            .rounded_full()
            .hover(|el| el.bg(rgb(0x121212)))
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

    fn toggle_pause_play(&mut self, cx: &mut Context<Self>) {
        if self.timer.read(cx).is_paused() {
            self.timer.update(cx, |e, cx| {
                e.play(cx);
            })
        } else {
            self.timer.update(cx, |e, cx| {
                e.pause(cx);
            })
        }
    }

    fn running_footer_row(&mut self, cx: &mut Context<Self>) -> Div {
        let play_pause_icon = if self.timer.read(cx).is_paused() {
            "icons/play.svg"
        } else {
            "icons/pause.svg"
        };

        return div()
            .flex()
            .flex_row()
            .p_4()
            .gap_2()
            .items_center()
            .justify_around()
            // PAUSE / PLAY
            .child(self.button(play_pause_icon, cx).on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|entity, _e, _w, cx| {
                    entity.toggle_pause_play(cx);
                }),
            ))
            // timeline
            .child(self.timeline.clone())
            // STOP
            .child(self.button("icons/stop.svg", cx).on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|entity, _e, _w, cx| {
                    entity.stop_and_reset(cx);
                }),
            ));
    }

    fn stop_and_reset(&mut self, cx: &mut Context<Self>) {
        self.session_index = 0;
        self.timer.update(cx, |t, cx| t.stop(cx));
        self.timeline.update(cx, |tl, _| {
            tl.active_index = 0;
            tl.current_progress = 0.;
        });
        cx.notify();
    }

    pub fn set_preset(&mut self, preset: TimerPreset, cx: &mut Context<Self>) {
        self.preset = preset;
        self.session_index = 0;
        self.timer.update(cx, |t, cx| {
            t.stop(cx);
        });
        let preset_ref = &self.preset;
        self.timeline.update(cx, |tl, _cx| {
            tl.active_index = 0;
            tl.current_progress = 0.;
            tl.update_segments(preset_ref);
        });
        cx.notify();
    }

    pub fn set_prefs(
        &mut self,
        auto_advance: bool,
        notifications_enabled: bool,
        sounds_enabled: bool,
        cx: &mut Context<Self>,
    ) {
        self.auto_advance = auto_advance;
        self.notifications_enabled = notifications_enabled;
        self.sounds_enabled = sounds_enabled;
        cx.notify();
    }

    fn start_timer(&mut self, cx: &mut Context<Self>) {
        let idx = self.session_index;
        let session = self.session().clone();
        let preset = self.preset.clone();

        self.timeline.update(cx, move |entity, _cx| {
            entity.update_segments(&preset);
            entity.active_index = idx;
            entity.current_progress = 0.;
        });

        self.timer.update(cx, |entity, cx| {
            entity.start(&session, cx);
        });
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
                            entity.start_timer(cx);
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

impl Render for TimerScreen {
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
            .child(
                TitleBar::new().child(div().child(title)).child(
                    div().flex().items_center().gap_2().child(
                        Button::new("settings")
                            .icon(IconName::Settings)
                            .ghost()
                            .on_click(cx.listener(|_this, _event, _window, cx| {
                                cx.emit(NavigationEvent {
                                    screen: Screen::Settings,
                                    timer_preset: None,
                                });
                            })),
                    ),
                ),
            )
            .child(self.app_container(cx));
    }
}
