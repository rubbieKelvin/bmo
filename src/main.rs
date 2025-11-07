use std::path::PathBuf;

use gpui::*;
mod app;
mod assets;
mod components;
mod constants;
mod session;
mod utils;

fn window_options(cx: &App) -> WindowOptions {
    let bounds = Bounds::centered(None, size(px(480.), px(480.)), cx);

    return WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some("Bmo".into()),
            // appears_transparent: true,
            ..Default::default()
        }),
        ..Default::default()
    };
}

fn main() {
    let app = Application::new().with_assets(assets::Assets {
        base: PathBuf::from("assets"),
    });

    app.run(|cx: &mut App| {
        // bring window to the foreground
        cx.activate(true);

        // close app when all windows are closed
        cx.on_window_closed(|cx| {
            if cx.windows().len() == 0 {
                cx.quit();
            }
        })
        .detach();

        cx.open_window(window_options(cx), |_, cx| {
            cx.new(|cx| app::BmoApp::new(cx))
        })
        .unwrap();
    });
}
