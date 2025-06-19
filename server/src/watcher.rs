use crate::{config::Config, parse::parse_org_file};
use notify::event::EventKind;
use notify::{RecommendedWatcher, Watcher};
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

    fn create_watcher()
    -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<notify::Event>>)> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let runtime = Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();

        let watcher = RecommendedWatcher::new(
            move |res| {
                let tx = tx.clone();
                runtime.spawn(async move {
                    if let Err(err) = tx.send(res).await {
                        error!("Failed to send file watcher event: {:?}", err);
                    }
                });
            },
            notify::Config::default(),
        )?;
        Ok((watcher, rx))
    }

    async fn watch_file(self, paths: Vec<String>) -> notify::Result<()> {
        let (mut watcher, mut rx) = Self::create_watcher()?;
        debug!("create watcher");

        for path in paths {
            match watcher.watch(path.as_ref(), notify::RecursiveMode::Recursive) {
                Ok(()) => {
                    debug!("start watch file: {:?}", path);
                }
                Err(err) => {
                    error!("Failed to watch path {}: {:?}", path, err);
                    return Err(err);
                }
            }
        }

        let mut prev_event = None;
        loop {
            let res = rx.recv().await;
            let Some(res) = res else {
                warn!("File watcher channel closed");
                break;
            };

            match res {
                Ok(event) => {
                    if let Some(old_event) = prev_event {
                        if old_event == event {
                            // same event skip
                            prev_event = Some(event.clone());
                            continue;
                        }
                    }
                    prev_event = Some(event.clone());
                    self.notify(&event).await;
                }
                Err(e) => {
                    error!("Error watching file: {:?}", e);
                }
            }
        }

        Ok(())
    }

    async fn notify(&self, event: &notify::Event) {
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                for p in &event.paths {
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
                }
            }
            _ => {
                // debug!("{:?}", event);
            }
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
