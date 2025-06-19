use anyhow::{Context as AnyhowContext, Result};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub async fn parse_org_file(path: &Path) -> Result<org_parser::Org> {
    let path_str = path.display().to_string();
    // debug!("Parsing org file: {}", path_str);

    let mut file = File::open(path)
        .await
        .with_context(|| format!("Failed to open file: {}", path_str))?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .await
        .with_context(|| format!("Failed to read file: {}", path_str))?;

    let content = std::str::from_utf8(&buf)
        .with_context(|| format!("Failed to parse UTF-8 content in file: {}", path_str))?;

    // debug!("File content length: {} bytes", content.len());

    let mut ctx = org_parser::Context::new();
    let mut org = org_parser::parse(&mut ctx, content)
        .with_context(|| format!("Failed to parse org content in file: {}", path_str))?;

    org.filename = Some(path_str.clone());
    // debug!("Successfully parsed org file: {}", path_str);
    Ok(org)
}
