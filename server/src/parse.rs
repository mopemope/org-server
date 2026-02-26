use anyhow::{Context as AnyhowContext, Result};
use std::path::Path;
use tokio::fs;

pub async fn parse_org_file(path: &Path) -> Result<org_parser::Org> {
    let path_str = path.display().to_string();

    let content = fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read file: {}", path_str))?;

    let mut ctx = org_parser::Context::new();
    let mut org = org_parser::parse(&mut ctx, &content)
        .with_context(|| format!("Failed to parse org content in file: {}", path_str))?;

    org.filename = Some(path_str.clone());
    Ok(org)
}
