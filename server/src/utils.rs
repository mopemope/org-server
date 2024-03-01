use anyhow::Context as _;
use anyhow::Result;
use std::path::PathBuf;

pub const APP_NAME: &str = "org-server";

pub fn get_config_file(name: &str) -> Result<PathBuf> {
    let xdg_dir =
        xdg::BaseDirectories::with_prefix(APP_NAME).context("failed get xdg directory")?;
    xdg_dir.place_config_file(name).context("failed get path")
}
