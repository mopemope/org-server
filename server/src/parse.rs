use anyhow::Result;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub async fn parse_org_file(path: &Path) -> Result<org_parser::Org> {
    let mut file = File::open(path).await?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await?;

    let content = std::str::from_utf8(&buf)?;
    let mut ctx = org_parser::Context::new();
    let mut org = org_parser::parse(&mut ctx, content)?;
    let p = format!("{}", path.display());
    org.filename = Some(p);
    Ok(org)
}
