use crate::{config::Config, parse::parse_org_file};
use anyhow::Result;
use chrono::{Local};
use org_parser::Remainder;
use std::time::Duration;
use tokio::{task, time};
use tracing::debug;
use walkdir::WalkDir;

async fn get_remainders(path: &str) -> Result<Vec<Remainder>> {
    let mut handles = vec![];

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path().to_owned();
        if let Some(ext) = path.extension() {
            if ext == "org" {
                let handle = task::spawn(async move { parse_org_file(&path).await });
                handles.push(handle);
            }
        }
    }

    let all_res = futures::future::join_all(handles).await;

    let mut res = vec![];

    for org in all_res.into_iter().flatten().flatten() {
        let mut remainders = org.get_remainders();
        res.append(&mut remainders);
    }
    Ok(res)
}

pub async fn check_remainders(config: &Config) -> Result<()> {
    let mut remainders = vec![];
    for p in &config.org_path {
        let mut res = get_remainders(p).await?;
        remainders.append(&mut res);
    }

    debug!("remainders: {:?}", remainders);

    let _forever = task::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;

            let now = Local::now().naive_local();
            let mut i = 0;
            while i < remainders.len() {
                if now > remainders[i].datetime {
                    let _val = remainders.remove(i);
                    // your code here
                } else {
                    i += 1;
                }
            }
            println!("checked");
        }
    });
    Ok(())
}
