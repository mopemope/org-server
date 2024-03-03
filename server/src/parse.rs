use anyhow::Result;
use std::time;
use tracing::debug;
use walkdir::{DirEntry, WalkDir};

pub fn parse_org(path: &str) -> Result<()> {
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "org" {
                let mut ctx = org_parser::Context::new();
                let now = time::Instant::now();
                let res = org_parser::parse_file(&mut ctx, path);
                debug!("{} {:?}", entry.path().display(), now.elapsed());
            }
        }
    }

    Ok(())
}
