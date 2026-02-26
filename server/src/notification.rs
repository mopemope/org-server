use anyhow::Result;
use notify_rust::Notification;

pub fn notify(summary: &str, body: &str) -> Result<()> {
    Notification::new()
        .summary(summary)
        .body(body)
        .icon("emacs")
        .appname("Emacs Reminder")
        .timeout(30000)
        .show()?;
    Ok(())
}
