use anyhow::Result;
use org_parser::{Org, Remainder};
use std::path::Path;
use tokio::io::AsyncReadExt;
use tokio::{fs::File, task};
use walkdir::WalkDir;

async fn parse_org_file(path: &Path) -> Result<org_parser::Org> {
    let mut file = File::open(path).await?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await?;

    let content = std::str::from_utf8(&buf)?;
    let mut ctx = org_parser::Context::new();
    let org = org_parser::parse(&mut ctx, content)?;
    Ok(org)
}

pub async fn parse_org_files(path: &str) -> Result<Vec<Org>> {
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

    let mut orgs = vec![];
    for org in all_res.into_iter().flatten().flatten() {
        orgs.push(org);
    }
    Ok(orgs)
}

pub async fn get_remainders(path: &str) -> Result<Vec<Remainder>> {
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
