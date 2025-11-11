use gpui::{
    App, AppContext, Context, Div, Entity, EventEmitter, ParentElement, Render, SharedString,
    Styled, Window, div,
};
use gpui_component::{
    Icon, IconName, IndexPath, TitleBar,
    button::{Button, ButtonVariants},
    label::Label,
    list::{ListDelegate, ListItem, ListState},
};

use crate::events::navigation::{NavigationEvent, Screen};

pub struct SettingScreen {
    preset_list: Entity<ListState<PresetListDelegate>>,
}

impl EventEmitter<NavigationEvent> for SettingScreen {}

impl SettingScreen {
    pub fn new(cx: &mut Context<Self>, window: &mut Window) -> Self {
        let preset_list = cx.new(|cx| {
            ListState::new(
                PresetListDelegate {
                    items: vec![PresetItem {
                        label: "Default".into(),
                        id: "".into(),
                        editable: false,
                    }],
                    selected_index: None,
                },
                window,
                cx,
            )
        });

        return SettingScreen { preset_list };
    }

    fn presets(&self) -> Div {
        return div()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .items_center()
                    .child(Label::new("Presets"))
                    .child(Button::new("new-preset-button").icon(IconName::Plus)),
            )
            .child(self.preset_list.clone());
    }

    fn body(&self) -> Div {
        return div().p_2().flex().flex_col().gap_4().child(self.presets());
    }
}

impl Render for SettingScreen {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        return div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                // title bar
                TitleBar::new().child(div().child("Settings")).child(
                    div().flex().items_center().gap_2().child(
                        Button::new("settings")
                            .icon(Icon::new(Icon::empty()).path("icons/x.svg"))
                            .ghost()
                            .on_click(cx.listener(|_this, _event, _window, cx| {
                                cx.emit(NavigationEvent {
                                    screen: Screen::Timer,
                                });
                            })),
                    ),
                ),
            )
            .child(self.body().flex_grow());
    }
}

#[allow(unused)]
struct PresetItem {
    id: SharedString,
    label: SharedString,
    editable: bool,
}

struct PresetListDelegate {
    items: Vec<PresetItem>,
    selected_index: Option<IndexPath>,
}

impl ListDelegate for PresetListDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.items.len()
    }

    fn render_item(
        &self,
        ix: IndexPath,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<Self::Item> {
        self.items.get(ix.row).map(|item| {
            ListItem::new(ix)
                .child(Label::new(item.label.clone()))
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
