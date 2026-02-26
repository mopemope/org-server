use crate::{config::Config, parse::parse_org_file};
use notify::RecommendedWatcher;
use notify_debouncer_mini::{DebouncedEvent, DebouncedEventKind, Debouncer, new_debouncer};
use org_parser::Org;
use tokio::runtime::Builder;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task;
use tracing::{debug, error, warn};

pub struct OrgWatcher {
    senders: Vec<Sender<Org>>,
}

impl OrgWatcher {
    pub const fn new(senders: Vec<Sender<Org>>) -> Self {
        Self { senders }
    }

    fn create_debouncer() -> notify::Result<(
        Debouncer<RecommendedWatcher>,
        Receiver<notify_debouncer_mini::DebounceEventResult>,
    )> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let runtime = Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(notify::Error::io)?;

        let debouncer = new_debouncer(
            std::time::Duration::from_millis(500),
            move |res: notify_debouncer_mini::DebounceEventResult| {
                let tx = tx.clone();
                runtime.spawn(async move {
                    if let Err(err) = tx.send(res).await {
                        error!("Failed to send file watcher event: {:?}", err);
                    }
                });
            },
        )?;
        Ok((debouncer, rx))
    }

    async fn watch_file(self, paths: Vec<String>) -> notify::Result<()> {
        let (mut debouncer, mut rx) = Self::create_debouncer()?;
        debug!("create debouncer");

        for path in paths {
            match debouncer
                .watcher()
                .watch(path.as_ref(), notify::RecursiveMode::Recursive)
            {
                Ok(()) => {
                    debug!("start watch file: {:?}", path);
                }
                Err(err) => {
                    error!("Failed to watch path {}: {:?}", path, err);
                    return Err(err);
                }
            }
        }

        loop {
            let res = rx.recv().await;
            let Some(res) = res else {
                warn!("File watcher channel closed");
                break;
            };

            match res {
                Ok(events) => {
                    for event in events {
                        self.notify(&event).await;
                    }
                }
                Err(e) => {
                    error!("Error watching file: {:?}", e);
                }
            }
        }

        Ok(())
    }

    async fn notify(&self, event: &DebouncedEvent) {
        let p = &event.path;
        match event.kind {
            DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous => {
                // If it is any modification, we try to parse it. If it fails due to file absence, we can treat it as remove
                if p.exists() {
                    match parse_org_file(p).await {
                        Ok(org) => {
                            let mut send_errors = 0;
                            for sender in &self.senders {
                                if let Err(err) = sender.send(org.clone()).await {
                                    send_errors += 1;
                                    debug!("Failed to send org data to channel: {:?}", err);
                                }
                            }
                            if send_errors > 0 {
                                warn!(
                                    "Failed to send to {}/{} channels for file: {}",
                                    send_errors,
                                    self.senders.len(),
                                    p.display()
                                );
                            }
                        }
                        Err(err) => {
                            error!("Failed to parse org file {}: {:?}", p.display(), err);
                        }
                    }
                } else {
                    let org = Org {
                        filename: Some(p.to_string_lossy().to_string()),
                        ..Default::default()
                    };
                    // Send an empty Org with just the filename to indicate deletion
                    for sender in &self.senders {
                        let _ = sender.send(org.clone()).await;
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn watch_files(config: &Config, tx: Vec<Sender<Org>>) {
    let paths = config.org_path.clone();
    let _forever = task::spawn(async move {
        let watcher = OrgWatcher::new(tx);
        if let Err(err) = watcher.watch_file(paths).await {
            error!("File watcher failed: {:?}", err);
        }
    });
}
