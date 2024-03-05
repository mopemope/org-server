use crate::{config::Config, parse::parse_org_file};
use anyhow::Result;
use notify::event::EventKind;
use notify::{RecommendedWatcher, Watcher};
use org_parser::Org;
use tokio::runtime::Builder;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task;
use tracing::{debug, error};

pub struct OrgWatcher {
    org_sender: Sender<Org>,
}

//
impl OrgWatcher {
    pub fn new(org_sender: Sender<Org>) -> Self {
        OrgWatcher { org_sender }
    }

    fn create_watcher(
        &self,
    ) -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<notify::Event>>)> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let runtime = Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();

        let watcher = RecommendedWatcher::new(
            move |res| {
                runtime.block_on(async {
                    tx.send(res).await.unwrap();
                })
            },
            notify::Config::default(),
        )?;
        Ok((watcher, rx))
    }

    async fn watch_file(self, paths: Vec<String>) -> notify::Result<()> {
        let (mut watcher, mut rx) = self.create_watcher()?;
        debug!("create watcher");

        for path in paths {
            watcher.watch(path.as_ref(), notify::RecursiveMode::Recursive)?;
            debug!("start watch file: {:?}", path);
        }

        let mut prev_event = None;
        loop {
            let res = rx.recv().await;
            let Some(res) = res else {
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
            EventKind::Create(_) => {
                //
            }
            EventKind::Modify(_data) => {
                for p in &event.paths {
                    if let Ok(org) = parse_org_file(p).await {
                        if let Err(err) = self.org_sender.send(org).await {
                            error!("SendError: {:?}", err);
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

pub fn watch_files(config: &Config, tx: Sender<Org>) -> Result<()> {
    let paths = config.org_path.clone();
    let _forever = task::spawn(async move {
        let watcher = OrgWatcher::new(tx);
        let _ = watcher.watch_file(paths).await;
    });

    Ok(())
}
