use gpui::*;
use gpui_component::{Root, TitleBar};

use crate::{app::BmoApp, assets::Assets};
mod app;
mod assets;
mod components;
mod constants;
mod session;

fn window_options(cx: &App) -> WindowOptions {
    let bounds = Bounds::centered(None, size(px(600.), px(450.)), cx);

    return WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitleBar::title_bar_options()),
        ..Default::default()
    };
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        cx.activate(true);

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
                let view = cx.new(|cx| BmoApp::new(cx));
                cx.new(|cx| Root::new(view.into(), window, cx))
            })?;

            return Ok(());
        })
        .detach();
    });
}
