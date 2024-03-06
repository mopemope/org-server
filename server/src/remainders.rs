use crate::{config::Config, notification, parse::parse_org_file};
use anyhow::Result;
use chrono::Local;
use org_parser::{Org, Remainder};
use std::{collections::HashSet, time::Duration};
use tokio::{sync::mpsc, task, time};
use tracing::{debug, error};
use walkdir::WalkDir;

async fn scan_remainders(path: &str, tx: mpsc::Sender<Org>) -> Result<()> {
    let mut n = 0;
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path().to_owned();
        if let Some(ext) = path.extension() {
            if ext == "org" {
                match parse_org_file(&path).await {
                    Ok(org) => {
                        if let Err(err) = tx.send(org).await {
                            error!("SendError: {:?}", err);
                        } else {
                            n += 1;
                        }
                    }
                    Err(err) => {
                        error!("ParseError: {:?}", err);
                    }
                }
            }
        }
    }
    debug!("scan: {:?} {} org files", path, n);
    Ok(())
}

pub fn scan(config: &Config, tx: mpsc::Sender<Org>) -> Result<()> {
    for p in &config.org_path {
        let p = p.clone();
        let tx = tx.clone();
        let _ = task::spawn(async move {
            if let Err(err) = scan_remainders(&p, tx).await {
                error!("ParseError {:?}", err);
            }
        });
    }
    Ok(())
}

pub async fn start_check(mut rx: mpsc::Receiver<Org>) -> Result<()> {
    let _forever = task::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(5));
        // let mut remainders: Vec<Remainder> = vec![];
        let mut remainders: HashSet<Remainder> = HashSet::new();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // debug!("start check");
                    let now = Local::now().naive_local();
                    let mut temp = vec![];
                    for val in &remainders {
                        if now > val.datetime {
                            // notify
                            let _ = notification::notify("Emacs Org Remainder", &val.title);
                            debug!("notify : {:?}", val);
                            temp.push(val.clone());  // remove entry
                        }
                    }

                    for val in temp {
                        remainders.remove(&val);
                    }

                    if !remainders.is_empty(){
                        debug!("remainder size: {:?}", remainders.len());
                    }

                }
                data = rx.recv() => {
                    if let Some(org) = data {
                        let res = org.get_remainders();
                        if !res.is_empty() {
                            let now = Local::now().naive_local();
                            for r in res {
                                if now < r.datetime {
                                    let dr = r.clone();
                                    if remainders.insert(r) {
                                        debug!("append remainder: {:?}", &dr);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });
    Ok(())
}
