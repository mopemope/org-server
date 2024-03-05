use anyhow::Result;
use notify_rust::Notification;

pub fn notify(summary: &str, body: &str) -> Result<()> {
    Notification::new()
        .summary(summary)
        .body(body)
        .icon("emacs")
        .appname("Emacs Remainder")
        .timeout(0)
        .show()?;
    Ok(())
}
