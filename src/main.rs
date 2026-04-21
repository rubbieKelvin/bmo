use crate::{app::BmoApp, assets::Assets};
use gpui::*;
use gpui_component::{Root, TitleBar};

mod app;
mod assets;
mod audio;
mod components;
mod constants;
mod db;
mod events;
mod notifications;
mod session;

const MIN_W: f32 = 800.;
const MIN_H: f32 = 650.;

fn window_options(cx: &App) -> WindowOptions {
    let bounds = Bounds::centered(None, size(px(MIN_W), px(MIN_H)), cx);

    return WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(Size {
            width: px(MIN_W),
            height: px(MIN_H),
        }),
        titlebar: Some(TitleBar::title_bar_options()),
        ..Default::default()
    };
}

fn main() {
    let appl = Application::new().with_assets(Assets);

    appl.run(move |cx| {
        cx.activate(true);
        gpui_component::init(cx);

        // close app when all windows are closed
        cx.on_window_closed(|cx| {
            if cx.windows().len() == 0 {
                cx.quit();
            }
        })
        .detach();

        let w_options = window_options(cx);
        cx.spawn(async move |cx| -> anyhow::Result<()> {
            cx.open_window(w_options, |window, cx| {
                let view = cx.new(|cx| BmoApp::new(cx, window));
                cx.new(|cx| Root::new(view, window, cx))
            })?;

            return Ok(());
        })
        .detach();
    });
}
