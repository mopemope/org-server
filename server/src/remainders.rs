use crate::{config::Config, notification, parse::parse_org_file};
use anyhow::Result;
use chrono::Local;
use org_parser::{Org, Remainder};
use std::time::Duration;
use tokio::{sync::mpsc, task, time};
use tracing::{debug, error};
use walkdir::WalkDir;

fn scan_remainders(path: &str, tx: mpsc::Sender<Org>) -> Result<()> {
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path().to_owned();
        if let Some(ext) = path.extension() {
            if ext == "org" {
                let tx = tx.clone();
                let _ = task::spawn(async move {
                    if let Ok(org) = parse_org_file(&path).await {
                        // debug!("org: {:?}", &org);
                        if let Err(err) = tx.send(org).await {
                            error!("SendError: {:?}", err);
                        }
                    }
                });
            }
        }
    }
    Ok(())
}
pub fn scan(config: &Config, tx: mpsc::Sender<Org>) -> Result<()> {
    for p in &config.org_path {
        scan_remainders(p, tx.clone())?;
    }
    Ok(())
}

pub async fn start_check(mut rx: mpsc::Receiver<Org>) -> Result<()> {
    let _forever = task::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(5));
        let mut remainders: Vec<Remainder> = vec![];

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    debug!("start check");
                    let now = Local::now().naive_local();
                    let mut i = 0;
                    while i < remainders.len() {
                        if now > remainders[i].datetime {
                            let val = remainders.remove(i);
                            if now > val.datetime {
                                // notify
                                let _ = notification::notify(&val.title, &val.title);
                                debug!("notify : {:?}", val);
                            }
                        } else {
                            i += 1;
                        }
                    }
                }
                data = rx.recv() => {
                    if let Some(org) = data {
                        let mut res = org.get_remainders();
                        if !res.is_empty() {
                            debug!("append remainders: {:?}", res);
                            remainders.append(&mut res);
                        }
                    }
                }
            }
        }
    });
    Ok(())
}

// pub async fn check_remainders2(config: &Config) -> Result<()> {
//     let mut remainders = vec![];
//     for p in &config.org_path {
//         let mut res = get_remainders(p).await?;
//         remainders.append(&mut res);
//     }

//     debug!("remainders: {:?}", remainders);

//     let _forever = task::spawn(async move {
//         let mut interval = time::interval(Duration::from_secs(5));

//         loop {
//             interval.tick().await;

//             let now = Local::now().naive_local();
//             let mut i = 0;
//             while i < remainders.len() {
//                 if now > remainders[i].datetime {
//                     let val = remainders.remove(i);
//                     if now > val.datetime {
//                         // notify
//                         let _ = notification::notify(&val.title, &val.title);
//                         debug!("notify : {:?}", val);
//                     }
//                 } else {
//                     i += 1;
//                 }
//             }
//             debug!("checked remainders: {}", remainders.len());
//         }
//     });
//     Ok(())
// }
