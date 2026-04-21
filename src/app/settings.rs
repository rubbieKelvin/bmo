use gpui::prelude::FluentBuilder;
use gpui::{
    App, AppContext, Context, Div, Entity, EventEmitter, ParentElement, Render, SharedString, Styled,
    Subscription, Window, div, px,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, IndexPath, TitleBar,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
    label::Label,
    list::{ListDelegate, ListEvent, ListItem, ListState},
};

use crate::db::{Database, Preset};
use crate::events::navigation::{NavigationEvent, Screen};

pub struct SettingScreen {
    pub preset_list: Entity<ListState<PresetListDelegate>>,
    db: Entity<Database>,
    new_preset_name: Entity<InputState>,
    #[allow(dead_code)]
    _db_obs: Subscription,
    #[allow(dead_code)]
    _list_sub: Subscription,
    #[allow(dead_code)]
    _name_input_sub: Subscription,
}

impl EventEmitter<NavigationEvent> for SettingScreen {}

impl SettingScreen {
    pub fn new(cx: &mut Context<Self>, window: &mut Window, db: Entity<Database>) -> Self {
        let presets = db.read(cx).presets().to_vec();
        let active_preset_id = db.read(cx).active_preset_id();

        let preset_list = cx.new(|cx| {
            ListState::new(
                PresetListDelegate {
                    presets,
                    active_preset_id,
                    selected_index: None,
                },
                window,
                cx,
            )
        });

        let new_preset_name = cx.new(|cx| {
            InputState::new(window, cx).placeholder("New preset name")
        });

        let _db_obs = cx.observe(&db, |this, _, cx| {
            let presets = this.db.read(cx).presets().to_vec();
            let active_preset_id = this.db.read(cx).active_preset_id();
            this.preset_list.update(cx, |list, cx| {
                let d = list.delegate_mut();
                d.presets = presets;
                d.active_preset_id = active_preset_id;
                cx.notify();
            });
        });

        let _list_sub = cx.subscribe(&preset_list, |this, list, event, cx| {
            if let ListEvent::Confirm(ix) = event {
                if let Some(p) = list.read(cx).delegate().presets.get(ix.row).cloned() {
                    if let Some(tp) = p.to_timer_preset() {
                        this.db.update(cx, |db, cx| {
                            db.set_active_preset_id(p.id);
                            db.schedule_persist_active_preset(cx);
                        });
                        cx.emit(NavigationEvent {
                            screen: Screen::Timer,
                            timer_preset: Some(tp),
                        });
                    }
                }
            }
        });

        let _name_input_sub = cx.subscribe_in(&new_preset_name, window, |this, _, ev, window, cx| {
            if matches!(ev, InputEvent::PressEnter { .. }) {
                this.submit_new_preset(window, cx);
            }
        });

        SettingScreen {
            preset_list,
            db,
            new_preset_name,
            _db_obs,
            _list_sub,
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

    fn presets_section(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .child(Label::new("Presets")),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        "Name your preset, then press Add or Enter (classic Pomodoro segments). Tap a row to set it as the timer preset and return to the timer.",
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
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .flex_grow()
                    .min_h(px(0.))
                    .child(Label::new("Your presets"))
                    .child(
                        div()
                            .flex_grow()
                            .min_h(px(160.))
                            .child(self.preset_list.clone()),
                    ),
            )
    }

    fn body(&self, cx: &mut Context<Self>) -> Div {
        div()
            .p_2()
            .flex()
            .flex_col()
            .gap_4()
            .flex_grow()
            .min_h(px(0.))
            .child(self.presets_section(cx).flex_grow())
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
                        Button::new("settings")
                            .icon(Icon::new(Icon::empty()).path("icons/x.svg"))
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

struct PresetListDelegate {
    presets: Vec<Preset>,
    active_preset_id: Option<i64>,
    selected_index: Option<IndexPath>,
}

impl ListDelegate for PresetListDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.presets.len()
    }

    fn render_item(
        &self,
        ix: IndexPath,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<Self::Item> {
        self.presets.get(ix.row).map(|preset| {
            let is_current = Some(preset.id) == self.active_preset_id;
            let row = div()
                .flex()
                .flex_row()
                .gap_2()
                .items_center()
                .when(is_current, |d| {
                    d.child(Icon::new(IconName::Check).size_4())
                        .child(div().text_xs().child("Current"))
                })
                .child(Label::new(SharedString::from(preset.name.clone())));

            ListItem::new(ix)
                .child(row)
                .selected(Some(ix) == self.selected_index)
        })
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }
}
