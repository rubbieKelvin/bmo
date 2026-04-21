use gpui::{
    AppContext, Context, Entity, ParentElement, Render, Styled, Subscription, Window, div,
};
use gpui_component::theme::{Theme, ThemeMode};

use crate::app::preset_editor::PresetEditorScreen;
use crate::app::timer::TimerScreen;
use crate::db::Database;
use crate::events::navigation::{NavigationEvent, Screen};

mod preset_editor;
mod settings;
mod timer;

pub struct BmoApp {
    current_screen: Screen,
    db: Entity<Database>,
    timer_screen: Entity<TimerScreen>,
    setting_screen: Entity<settings::SettingScreen>,
    preset_editor: Option<Entity<PresetEditorScreen>>,
    last_applied_active_preset_id: Option<i64>,
    pending_initial_db_timer_sync: bool,
    #[allow(dead_code)]
    _db_sync: Subscription,
    #[allow(dead_code)]
    _timer_nav: Subscription,
    #[allow(dead_code)]
    _settings_nav: Subscription,
}

impl BmoApp {
    pub fn new(cx: &mut Context<Self>, window: &mut Window) -> Self {
        let db = cx.new(|_cx| Database::new());
        let timer_screen = cx.new(|cx| TimerScreen::new(cx));
        let setting_screen = cx.new(|cx| settings::SettingScreen::new(cx, window, db.clone()));

        db.update(cx, |this, cx| {
            this.init(cx);
        });

        // Timer -> (settings navigation only; no window-bound construction needed)
        let _timer_nav = cx.subscribe(
            &timer_screen,
            |parent, _entity, event: &NavigationEvent, context| {
                parent.set_screen_no_window(event.screen.clone(), context);
            },
        );

        // Settings -> (may open PresetEditor, which needs a Window)
        let _settings_nav = cx.subscribe_in(&setting_screen, window, {
            let db = db.clone();
            let timer_screen = timer_screen.clone();
            move |parent, _entity, event: &NavigationEvent, window, context| {
                parent.set_screen(event.screen.clone(), window, context);
                db.update(context, |this, cx| {
                    this.update_preset_list(cx);
                });
                if let Some(preset) = event.timer_preset.clone() {
                    timer_screen.update(context, |t, cx| {
                        t.set_preset(preset, cx);
                    });
                }
            }
        });

        let _db_sync = cx.observe_in(&db, window, |app, db_ent, window, cx| {
            let db_ref = db_ent.read(cx);
            let id = db_ref.active_preset_id();
            let prefs = db_ref.prefs().clone();

            // Apply theme from persisted prefs.
            let desired = if prefs.theme == "light" {
                ThemeMode::Light
            } else {
                ThemeMode::Dark
            };
            Theme::change(desired, Some(window), cx);

            app.timer_screen.update(cx, |t, cx| {
                t.set_prefs(
                    prefs.auto_advance,
                    prefs.notifications_enabled,
                    prefs.sounds_enabled,
                    cx,
                );
            });

            if !app.pending_initial_db_timer_sync && id == app.last_applied_active_preset_id {
                return;
            }
            app.pending_initial_db_timer_sync = false;
            app.last_applied_active_preset_id = id;

            if let Some(pid) = id {
                let preset = db_ent
                    .read(cx)
                    .presets()
                    .iter()
                    .find(|p| p.id == pid)
                    .and_then(|p| p.to_timer_preset());
                if let Some(tp) = preset {
                    app.timer_screen.update(cx, |t, cx| t.set_preset(tp, cx));
                }
            }
        });

        return Self {
            current_screen: Screen::Timer,
            db,
            timer_screen,
            setting_screen,
            preset_editor: None,
            last_applied_active_preset_id: None,
            pending_initial_db_timer_sync: true,
            _db_sync,
            _timer_nav,
            _settings_nav,
        };
    }

    /// Screen transitions that never mount an editor (i.e. only Timer or
    /// Settings) can be done without a window reference.
    fn set_screen_no_window(&mut self, screen: Screen, cx: &mut Context<Self>) {
        debug_assert!(!matches!(screen, Screen::PresetEditor(_)));
        self.current_screen = screen;
        self.preset_editor = None;
        cx.notify();
    }

    fn set_screen(&mut self, screen: Screen, window: &mut Window, cx: &mut Context<Self>) {
        self.current_screen = screen.clone();

        match screen {
            Screen::Settings => {
                self.db.update(cx, |this, cx| this.update_preset_list(cx));
                self.preset_editor = None;
            }
            Screen::PresetEditor(preset_id) => {
                let db = self.db.clone();
                let editor = cx.new(|cx| PresetEditorScreen::new(cx, window, db, preset_id));

                cx.subscribe(&editor, |parent, _e, event: &NavigationEvent, cx| {
                    // Editor only ever navigates back to Settings.
                    parent.set_screen_no_window(event.screen.clone(), cx);
                })
                .detach();

                self.preset_editor = Some(editor);
            }
            Screen::Timer => {
                self.preset_editor = None;
            }
        };
        cx.notify();
    }
}

impl Render for BmoApp {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        return match &self.current_screen {
            Screen::Timer => div().size_full().child(self.timer_screen.clone()),
            Screen::Settings => div().size_full().child(self.setting_screen.clone()),
            Screen::PresetEditor(_) => match self.preset_editor.clone() {
                Some(e) => div().size_full().child(e),
                None => div().size_full().child(self.setting_screen.clone()),
            },
        };
    }
}
