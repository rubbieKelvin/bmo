use gpui::{
    AppContext, Context, Div, Entity, EventEmitter, ParentElement, Render, SharedString, Styled,
    Subscription, Window, div, px,
};
use gpui_component::{
    ActiveTheme as _, Disableable, IconName, StyledExt, TitleBar,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
    label::Label,
};

use crate::db::{Database, Preset, Session, SessionType};
use crate::events::navigation::{NavigationEvent, Screen};

/// Local, editable row backing a `Session`. Changes persist via
/// `Database::update_session` on blur/enter, so there is no explicit
/// "save" step - "Back" just leaves the screen.
struct SessionRow {
    id: i64,
    session_type: SessionType,
    name_state: Entity<InputState>,
    duration_state: Entity<InputState>,
    #[allow(dead_code)]
    _name_sub: Subscription,
    #[allow(dead_code)]
    _duration_sub: Subscription,
}

pub struct PresetEditorScreen {
    db: Entity<Database>,
    preset_id: i64,
    title_state: Entity<InputState>,
    rows: Vec<SessionRow>,
    missing: bool,
    #[allow(dead_code)]
    _db_obs: Subscription,
    #[allow(dead_code)]
    _title_sub: Subscription,
}

impl EventEmitter<NavigationEvent> for PresetEditorScreen {}

fn format_duration(secs: i64) -> String {
    let s = secs.max(0);
    format!("{:02}:{:02}", s / 60, s % 60)
}

/// Parse "mm:ss" or a bare integer (interpreted as minutes) into seconds.
fn parse_duration(raw: &str) -> Option<i64> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    if let Some((m, rest)) = s.split_once(':') {
        let m: i64 = m.trim().parse().ok()?;
        let s: i64 = rest.trim().parse().ok()?;
        if !(0..60).contains(&s) || m < 0 {
            return None;
        }
        return Some(m * 60 + s);
    }
    let minutes: i64 = s.parse().ok()?;
    if minutes < 0 {
        return None;
    }
    Some(minutes * 60)
}

impl PresetEditorScreen {
    pub fn new(
        cx: &mut Context<Self>,
        window: &mut Window,
        db: Entity<Database>,
        preset_id: i64,
    ) -> Self {
        let preset = db
            .read(cx)
            .presets()
            .iter()
            .find(|p| p.id == preset_id)
            .cloned();

        let title_initial = preset.as_ref().map(|p| p.name.clone()).unwrap_or_default();
        let title_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Preset name")
                .default_value(SharedString::from(title_initial))
        });

        let _title_sub = cx.subscribe(&title_state, move |this, entity, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                let name = entity.read(cx).value().to_string();
                this.db.update(cx, |db, cx| {
                    db.rename_preset(this.preset_id, name, cx);
                });
            }
        });

        let rows: Vec<SessionRow> = preset
            .as_ref()
            .map(|p| {
                p.sessions
                    .iter()
                    .map(|s| Self::build_row(cx, window, s.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let _db_obs = cx.observe_in(&db, window, |this, db_ent, window, cx| {
            let preset = db_ent
                .read(cx)
                .presets()
                .iter()
                .find(|p| p.id == this.preset_id)
                .cloned();
            match preset {
                None => {
                    this.missing = true;
                    cx.notify();
                }
                Some(p) => {
                    this.sync_from_preset(&p, window, cx);
                    cx.notify();
                }
            }
        });

        Self {
            db,
            preset_id,
            title_state,
            rows,
            missing: preset.is_none(),
            _db_obs,
            _title_sub,
        }
    }

    pub fn preset_id(&self) -> i64 {
        self.preset_id
    }

    fn build_row(cx: &mut Context<Self>, window: &mut Window, s: Session) -> SessionRow {
        let id = s.id;
        let name_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Session name")
                .default_value(SharedString::from(s.name.clone()))
        });
        let duration_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("mm:ss")
                .default_value(SharedString::from(format_duration(s.duration_in_sec)))
        });

        let _name_sub = cx.subscribe(&name_state, move |this, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                this.persist_row(id, cx);
            }
        });
        let _duration_sub = cx.subscribe(&duration_state, move |this, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                this.persist_row(id, cx);
            }
        });

        SessionRow {
            id: s.id,
            session_type: s.session_type,
            name_state,
            duration_state,
            _name_sub,
            _duration_sub,
        }
    }

    /// Reconcile row list against the latest preset from the DB. Existing
    /// `InputState` entities are preserved so in-flight edits are not lost;
    /// new rows get newly-constructed `InputState`s using the provided window.
    fn sync_from_preset(&mut self, preset: &Preset, window: &mut Window, cx: &mut Context<Self>) {
        self.missing = false;
        let live_ids: std::collections::HashSet<i64> =
            preset.sessions.iter().map(|s| s.id).collect();
        self.rows.retain(|r| live_ids.contains(&r.id));

        for s in preset.sessions.iter() {
            if let Some(r) = self.rows.iter_mut().find(|r| r.id == s.id) {
                r.session_type = s.session_type;
            }
        }

        let mut ordered: Vec<SessionRow> = Vec::with_capacity(preset.sessions.len());
        for s in preset.sessions.iter() {
            if let Some(pos) = self.rows.iter().position(|r| r.id == s.id) {
                ordered.push(self.rows.swap_remove(pos));
            } else {
                ordered.push(Self::build_row(cx, window, s.clone()));
            }
        }
        self.rows = ordered;

        // Keep the preset title input in sync only if the user is not actively editing.
        let current_title = self.title_state.read(cx).value().to_string();
        if current_title != preset.name {
            self.title_state.update(cx, |st, cx| {
                st.set_value(SharedString::from(preset.name.clone()), window, cx);
            });
        }
    }

    fn persist_row(&self, id: i64, cx: &mut Context<Self>) {
        let Some(row) = self.rows.iter().find(|r| r.id == id) else {
            return;
        };
        let name = row.name_state.read(cx).value().to_string();
        let raw_dur = row.duration_state.read(cx).value().to_string();
        let Some(secs) = parse_duration(&raw_dur) else {
            return;
        };
        let kind = row.session_type;
        self.db.update(cx, |db, cx| {
            db.update_session(id, name, secs.max(1), kind, cx);
        });
    }

    fn toggle_row_type(&mut self, id: i64, cx: &mut Context<Self>) {
        let (name, secs, kind) = {
            let Some(row) = self.rows.iter_mut().find(|r| r.id == id) else {
                return;
            };
            row.session_type = match row.session_type {
                SessionType::Focus => SessionType::Break,
                SessionType::Break => SessionType::Focus,
            };
            let name = row.name_state.read(cx).value().to_string();
            let raw = row.duration_state.read(cx).value().to_string();
            let secs = parse_duration(&raw).unwrap_or(60);
            (name, secs, row.session_type)
        };
        self.db.update(cx, |db, cx| {
            db.update_session(id, name, secs.max(1), kind, cx);
        });
        cx.notify();
    }

    fn move_row(&mut self, id: i64, delta: isize, cx: &mut Context<Self>) {
        let Some(pos) = self.rows.iter().position(|r| r.id == id) else {
            return;
        };
        let new_pos = pos as isize + delta;
        if new_pos < 0 || new_pos >= self.rows.len() as isize {
            return;
        }
        self.rows.swap(pos, new_pos as usize);
        let ordered_ids: Vec<i64> = self.rows.iter().map(|r| r.id).collect();
        let preset_id = self.preset_id;
        self.db.update(cx, |db, cx| {
            db.reorder_sessions(preset_id, ordered_ids, cx);
        });
        cx.notify();
    }

    fn delete_row(&mut self, id: i64, cx: &mut Context<Self>) {
        self.rows.retain(|r| r.id != id);
        self.db.update(cx, |db, cx| {
            db.delete_session(id, cx);
        });
        cx.notify();
    }

    fn add_session(&mut self, cx: &mut Context<Self>) {
        let preset_id = self.preset_id;
        // The `observe_in` hook mounted in `new` will rebuild rows (with
        // InputStates) once `update_preset_list` completes the round-trip.
        self.db.update(cx, |db, cx| {
            db.add_session(
                preset_id,
                "New session".to_string(),
                25 * 60,
                SessionType::Focus,
                cx,
            );
        });
    }

    fn delete_preset(&mut self, cx: &mut Context<Self>) {
        let id = self.preset_id;
        self.db.update(cx, |db, cx| {
            db.soft_delete_preset(id, cx);
        });
        cx.emit(NavigationEvent::goto(Screen::Settings));
    }

    fn duplicate_preset(&mut self, cx: &mut Context<Self>) {
        let id = self.preset_id;
        self.db.update(cx, |db, cx| {
            db.duplicate_preset(id, cx);
        });
        cx.emit(NavigationEvent::goto(Screen::Settings));
    }

    fn back(&mut self, cx: &mut Context<Self>) {
        cx.emit(NavigationEvent::goto(Screen::Settings));
    }

    fn row_view(&self, row: &SessionRow, index: usize, cx: &mut Context<Self>) -> Div {
        let id = row.id;
        let is_focus = matches!(row.session_type, SessionType::Focus);
        let type_label = if is_focus { "Focus" } else { "Break" };
        let last = index + 1 == self.rows.len();

        div()
            .flex()
            .flex_row()
            .gap_2()
            .items_center()
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .flex_grow()
                    .min_w(px(80.))
                    .child(Input::new(&row.name_state)),
            )
            .child(
                div()
                    .w(px(80.))
                    .child(Input::new(&row.duration_state)),
            )
            .child(
                Button::new(("type-toggle", id as usize))
                    .label(type_label)
                    .ghost()
                    .on_click(cx.listener(move |this, _e, _w, cx| {
                        this.toggle_row_type(id, cx);
                    })),
            )
            .child(
                Button::new(("up", id as usize))
                    .icon(IconName::ChevronUp)
                    .ghost()
                    .disabled(index == 0)
                    .on_click(cx.listener(move |this, _e, _w, cx| {
                        this.move_row(id, -1, cx);
                    })),
            )
            .child(
                Button::new(("down", id as usize))
                    .icon(IconName::ChevronDown)
                    .ghost()
                    .disabled(last)
                    .on_click(cx.listener(move |this, _e, _w, cx| {
                        this.move_row(id, 1, cx);
                    })),
            )
            .child(
                Button::new(("del", id as usize))
                    .icon(IconName::Delete)
                    .ghost()
                    .danger()
                    .on_click(cx.listener(move |this, _e, _w, cx| {
                        this.delete_row(id, cx);
                    })),
            )
    }

    fn body(&self, cx: &mut Context<Self>) -> Div {
        if self.missing {
            return div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .flex_grow()
                .p_4()
                .gap_2()
                .child(Label::new("This preset was deleted or cannot be found."))
                .child(
                    Button::new("back")
                        .label("Back to settings")
                        .on_click(cx.listener(|this, _, _, cx| this.back(cx))),
                );
        }

        let rows = self
            .rows
            .iter()
            .enumerate()
            .map(|(i, r)| self.row_view(r, i, cx))
            .collect::<Vec<_>>();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .flex_grow()
            .min_h(px(0.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .items_center()
                    .child(Label::new("Name"))
                    .child(
                        div()
                            .flex_grow()
                            .child(Input::new(&self.title_state)),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        "Sessions run top-to-bottom. Press Enter or click away to save a field. Use the arrows to reorder.",
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .flex_grow()
                    .min_h(px(0.))
                    .scrollable(gpui_component::scroll::ScrollbarAxis::Vertical)
                    .children(rows),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .items_center()
                    .child(
                        Button::new("add-session")
                            .label("Add session")
                            .primary()
                            .on_click(cx.listener(|this, _, _, cx| this.add_session(cx))),
                    )
                    .child(
                        Button::new("duplicate")
                            .label("Duplicate preset")
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| this.duplicate_preset(cx))),
                    )
                    .child(
                        Button::new("delete-preset")
                            .label("Delete preset")
                            .danger()
                            .on_click(cx.listener(|this, _, _, cx| this.delete_preset(cx))),
                    ),
            )
    }
}

impl Render for PresetEditorScreen {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .min_h(px(0.))
            .child(
                TitleBar::new()
                    .child(div().child("Edit preset"))
                    .child(
                        div().flex().items_center().gap_2().child(
                            Button::new("editor-back")
                                .icon(IconName::ArrowLeft)
                                .ghost()
                                .on_click(cx.listener(|this, _, _, cx| this.back(cx))),
                        ),
                    ),
            )
            .child(self.body(cx).flex_grow())
    }
}
