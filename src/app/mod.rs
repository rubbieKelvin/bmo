use gpui::{AppContext, Context, Entity, ParentElement, Render, Styled, Subscription, Window, div};

use crate::app::timer::TimerScreen;
use crate::db::Database;
use crate::events::navigation::{NavigationEvent, Screen};

mod settings;
mod timer;

pub struct BmoApp {
    current_screen: Screen,
    db: Entity<Database>,
    timer_screen: Entity<TimerScreen>,
    setting_screen: Entity<settings::SettingScreen>,
    last_applied_active_preset_id: Option<i64>,
    pending_initial_db_timer_sync: bool,
    #[allow(dead_code)]
    _db_timer_sync: Subscription,
}

impl BmoApp {
    pub fn new(cx: &mut Context<Self>, window: &mut Window) -> Self {
        let db = cx.new(|_cx| Database::new());
        let timer_screen = cx.new(|cx| TimerScreen::new(cx));
        let setting_screen = cx.new(|cx| settings::SettingScreen::new(cx, window, db.clone()));

        db.update(cx, |this, cx| {
            this.init(cx);
        });

        // When we click settings on the timer app, show the settings page
        cx.subscribe(
            &timer_screen,
            |parent, _entity, event: &NavigationEvent, context| {
                parent.set_screen(event.screen.clone(), context);
            },
        )
        .detach();

        cx.subscribe(&setting_screen, {
            let db = db.clone();
            let timer_screen = timer_screen.clone();
            move |parent, _entity, event: &NavigationEvent, context| {
                parent.set_screen(event.screen.clone(), context);
                db.update(context, |this, cx| {
                    this.update_preset_list(cx);
                });
                if let Some(preset) = event.timer_preset.clone() {
                    timer_screen.update(context, |t, cx| {
                        t.set_preset(preset, cx);
                    });
                }
            }
        })
        .detach();

        let _db_timer_sync = cx.observe(&db, |app, db_ent, cx| {
            let id = db_ent.read(cx).active_preset_id();
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
            last_applied_active_preset_id: None,
            pending_initial_db_timer_sync: true,
            _db_timer_sync,
        };
    }

    fn set_screen(&mut self, screen: Screen, cx: &mut Context<Self>) {
        self.current_screen = screen.clone();

        match screen {
            Screen::Settings => {
                self.db.update(cx, |this, cx| this.update_preset_list(cx));
            }
            _ => (),
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
        return match self.current_screen {
            Screen::Timer => div().size_full().child(self.timer_screen.clone()),
            Screen::Settings => div().size_full().child(self.setting_screen.clone()),
        };
    }
}
