use crate::{config::Config, notification, parse::parse_org_file};
use anyhow::Result;
use chrono::Local;
use org_parser::{Org, Reminder};
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use tokio::{sync::mpsc, task, time};
use tracing::{debug, error, warn};
use walkdir::WalkDir;

async fn scan_reminders(path: &str, tx: mpsc::Sender<Org>) -> Result<()> {
    let now = Instant::now();
    let mut n = 0;
    let mut errors = 0;

    debug!("Starting scan of path: {}", path);

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let path = entry.path().to_owned();
        if let Some(ext) = path.extension() {
            if ext == "org" {
                match parse_org_file(&path).await {
                    Ok(org) => {
                        if let Err(err) = tx.send(org).await {
                            error!(
                                "Failed to send parsed org data for file {}: {:?}",
                                path.display(),
                                err
                            );
                            // チャンネルが閉じられている場合は処理を中断
                            break;
                        } else {
                            n += 1;
                            //debug!("Successfully processed file: {}", path.display());
                        }
                    }
                    Err(err) => {
                        error!("Failed to parse org file {}: {:?}", path.display(), err);
                        errors += 1;
                    }
                }
            }
        }
    }

    if errors > 0 {
        warn!(
            "Scan completed with {} errors out of {} total org files",
            errors,
            n + errors
        );
    } else {
        debug!(
            "Scan completed successfully: {} org files processed in {:?}",
            n,
            now.elapsed()
        );
    }

    Ok(())
}

pub fn scan(config: &Config, tx: &mpsc::Sender<Org>) {
    let mut handles = Vec::new();

    for p in &config.org_path {
        let p = p.clone();
        let tx = tx.clone();
        let handle = task::spawn(async move {
            if let Err(err) = scan_reminders(&p, tx).await {
                error!("Scan task failed for path {}: {:?}", p, err);
            }
        });
        handles.push(handle);
    }

    debug!("Spawned {} scan tasks for org paths", handles.len());

    // バックグラウンドでタスクの完了を待つ
    task::spawn(async move {
        for (i, handle) in handles.into_iter().enumerate() {
            match handle.await {
                Ok(()) => debug!("Scan task {} completed successfully", i + 1),
                Err(err) => error!("Scan task {} join error: {:?}", i + 1, err),
            }
        }
        debug!("All scan tasks completed");
    });
}

pub fn start_check(mut rx: mpsc::Receiver<Org>) {
    let _forever = task::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(5));
        let mut reminders: HashSet<Reminder> = HashSet::new();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // debug!("start check");
                    let now = Local::now().naive_local();
                    let mut temp = vec![];
                    for val in &reminders {
                        if now > val.datetime {
                            // notify
                            let _ = notification::notify("Emacs Org Remainder", &val.title);
                            debug!("notify : {:?}", val);
                            temp.push(val.clone());  // remove entry
                        }
                    }
                    for val in temp {
                        reminders.remove(&val);
                    }

                }
                data = rx.recv() => {
                    match data {
                        Some(org) => {
                            let res = org.get_reminders();
                            if !res.is_empty() {
                                let now = Local::now().naive_local();
                                for r in res {
                                    if now < r.datetime {
                                        let dr = r.clone();
                                        if reminders.insert(r) {
                                            debug!("append reminder: {:?}", &dr);
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            debug!("Reminder channel closed, stopping reminder checker");
                            break;
                        }
                    }
                }
            }
        }
    });
}
