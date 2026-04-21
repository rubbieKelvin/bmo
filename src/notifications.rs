//! Thin wrapper around `notify-rust` so the timer can fire OS notifications
//! without knowing about the underlying crate.

/// Fire a native desktop notification. Failures are logged and swallowed so
/// that the timer loop is never disrupted by notification issues (for
/// example, a user disabling notifications at the OS level).
pub fn notify_segment(title: &str, body: &str) {
    let summary = format!("Bmo: {title}");
    let res = notify_rust::Notification::new()
        .appname("Bmo")
        .summary(&summary)
        .body(body)
        .show();
    if let Err(e) = res {
        eprintln!("bmo: notification failed: {e}");
    }
}
