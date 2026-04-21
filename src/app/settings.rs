use gpui::prelude::FluentBuilder;
use gpui::{
    App, AppContext, Context, Div, Entity, EventEmitter, ParentElement, Render, SharedString,
    Styled, Subscription, Window, div, px,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Sizable, StyledExt, TitleBar,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
    label::Label,
    switch::Switch,
    theme::{Theme, ThemeMode},
};

use crate::db::Database;
use crate::events::navigation::{NavigationEvent, Screen};

pub struct SettingScreen {
    db: Entity<Database>,
    new_preset_name: Entity<InputState>,
    #[allow(dead_code)]
    _db_obs: Subscription,
    #[allow(dead_code)]
    _name_input_sub: Subscription,
}

impl EventEmitter<NavigationEvent> for SettingScreen {}

impl SettingScreen {
    pub fn new(cx: &mut Context<Self>, window: &mut Window, db: Entity<Database>) -> Self {
        let new_preset_name =
            cx.new(|cx| InputState::new(window, cx).placeholder("New preset name"));

        let _db_obs = cx.observe(&db, |_this, _db_ent, cx| {
            cx.notify();
        });

        let _name_input_sub =
            cx.subscribe_in(&new_preset_name, window, |this, _, ev, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. }) {
                    this.submit_new_preset(window, cx);
                }
            });

        SettingScreen {
            db,
            new_preset_name,
            _db_obs,
            _name_input_sub,
        }
    }

    fn submit_new_preset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.new_preset_name.read(cx).value().to_string();
        self.db.update(cx, |db, cx| {
            db.create_preset(name, cx);
        });
        self.new_preset_name.update(cx, |input, cx| {
            input.set_value("", window, cx);
        });
    }

    fn use_preset(&mut self, preset_id: i64, cx: &mut Context<Self>) {
        let preset = self
            .db
            .read(cx)
            .presets()
            .iter()
            .find(|p| p.id == preset_id)
            .cloned();
        let Some(p) = preset else {
            return;
        };
        let tp = p.to_timer_preset();
        self.db.update(cx, |db, cx| {
            db.set_active_preset_id(p.id);
            db.schedule_persist_active_preset(cx);
        });
        cx.emit(NavigationEvent {
            screen: Screen::Timer,
            timer_preset: tp,
        });
    }

    fn edit_preset(&mut self, preset_id: i64, cx: &mut Context<Self>) {
        cx.emit(NavigationEvent {
            screen: Screen::PresetEditor(preset_id),
            timer_preset: None,
        });
    }

    fn preset_row(
        &self,
        id: i64,
        name: String,
        session_count: usize,
        is_active: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        div()
            .flex()
            .flex_row()
            .gap_2()
            .items_center()
            .w_full()
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().border)
            .when(is_active, |d| d.child(Icon::new(IconName::Check).size_4()))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_grow()
                    .child(Label::new(SharedString::from(name)))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "{} session{}",
                                session_count,
                                if session_count == 1 { "" } else { "s" }
                            )),
                    ),
            )
            .child(
                Button::new(("use", id as usize))
                    .label(if is_active { "In use" } else { "Use" })
                    .when(!is_active, |b| b.primary())
                    .when(is_active, |b| b.ghost())
                    .small()
                    .on_click(cx.listener(move |this, _e, _w, cx| {
                        this.use_preset(id, cx);
                    })),
            )
            .child(
                Button::new(("edit", id as usize))
                    .label("Edit")
                    .ghost()
                    .small()
                    .on_click(cx.listener(move |this, _e, _w, cx| {
                        this.edit_preset(id, cx);
                    })),
            )
    }

    fn presets_section(&self, cx: &mut Context<Self>) -> Div {
        // Snapshot the data we need so we don't hold a DB borrow across the
        // listener-building loop.
        let active = self.db.read(cx).active_preset_id();
        let preset_infos: Vec<(i64, String, usize)> = self
            .db
            .read(cx)
            .presets()
            .iter()
            .map(|p| (p.id, p.name.clone(), p.sessions.len()))
            .collect();

        let rows: Vec<Div> = preset_infos
            .into_iter()
            .map(|(id, name, count)| self.preset_row(id, name, count, Some(id) == active, cx))
            .collect();

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(Label::new("Presets"))
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        "Use runs the preset now. The pencil icon opens the session editor.",
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .items_center()
                    .child(Input::new(&self.new_preset_name).cleanable(true).flex_grow())
                    .child(
                        Button::new("add-preset-submit")
                            .label("Add")
                            .primary()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.submit_new_preset(window, cx);
                            })),
                    ),
            )
            .child(div().flex().flex_col().gap_2().children(rows))
    }

    fn general_section(&self, cx: &mut Context<Self>) -> Div {
        let prefs = self.db.read(cx).prefs().clone();

        div()
            .flex()
            .flex_col()
            .gap_2()
            .pt_2()
            .child(Label::new("General"))
            .child(self.toggle_row(
                "auto-advance",
                "Auto-advance segments",
                "Start the next session automatically when a segment finishes.",
                prefs.auto_advance,
                cx.listener(|this, v: &bool, _w, cx| {
                    let v = *v;
                    this.db.update(cx, |db, cx| db.set_auto_advance(v, cx));
                }),
                cx,
            ))
            .child(self.toggle_row(
                "notifications-enabled",
                "Desktop notifications",
                "Show an OS notification on segment completion.",
                prefs.notifications_enabled,
                cx.listener(|this, v: &bool, _w, cx| {
                    let v = *v;
                    this.db
                        .update(cx, |db, cx| db.set_notifications_enabled(v, cx));
                }),
                cx,
            ))
            .child(self.toggle_row(
                "sounds-enabled",
                "Sound cues",
                "Play a short sound on segment completion.",
                prefs.sounds_enabled,
                cx.listener(|this, v: &bool, _w, cx| {
                    let v = *v;
                    this.db.update(cx, |db, cx| db.set_sounds_enabled(v, cx));
                }),
                cx,
            ))
            .child(self.theme_row(&prefs.theme, cx))
    }

    fn toggle_row<F>(
        &self,
        id: &'static str,
        title: &'static str,
        subtitle: &'static str,
        checked: bool,
        on_change: F,
        cx: &mut Context<Self>,
    ) -> Div
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap_2()
            .py_1()
            .child(
                div()
                    .flex_grow()
                    .flex()
                    .flex_col()
                    .child(Label::new(title))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(subtitle),
                    ),
            )
            .child(Switch::new(id).checked(checked).on_click(on_change))
    }

    fn theme_row(&self, current_theme: &str, cx: &mut Context<Self>) -> Div {
        let is_dark = current_theme == "dark";
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap_2()
            .py_1()
            .child(Label::new("Theme"))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_1()
                    .child(
                        Button::new("theme-light")
                            .icon(IconName::Sun)
                            .label("Light")
                            .when(!is_dark, |b| b.primary())
                            .when(is_dark, |b| b.ghost())
                            .on_click(cx.listener(|this, _e, window, cx| {
                                Theme::change(ThemeMode::Light, Some(window), cx);
                                this.db
                                    .update(cx, |db, cx| db.set_theme("light".into(), cx));
                            })),
                    )
                    .child(
                        Button::new("theme-dark")
                            .icon(IconName::Moon)
                            .label("Dark")
                            .when(is_dark, |b| b.primary())
                            .when(!is_dark, |b| b.ghost())
                            .on_click(cx.listener(|this, _e, window, cx| {
                                Theme::change(ThemeMode::Dark, Some(window), cx);
                                this.db
                                    .update(cx, |db, cx| db.set_theme("dark".into(), cx));
                            })),
                    ),
            )
    }

    fn body(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex_grow()
            .min_h(px(0.))
            .child(
                div()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .size_full()
                    .scrollable(gpui_component::scroll::ScrollbarAxis::Vertical)
                    .child(self.presets_section(cx))
                    .child(self.general_section(cx)),
            )
    }
}

impl Render for SettingScreen {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .min_h(px(0.))
            .child(
                TitleBar::new().child(div().child("Settings")).child(
                    div().flex().items_center().gap_2().child(
                        Button::new("settings-close")
                            .icon(IconName::Close)
                            .ghost()
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.emit(NavigationEvent {
                                    screen: Screen::Timer,
                                    timer_preset: None,
                                });
                            })),
                    ),
                ),
            )
            .child(self.body(cx).flex_grow())
    }
}
