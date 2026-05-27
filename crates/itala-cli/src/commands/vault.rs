//! Vault command handlers.

use std::sync::Arc;

use anyhow::Context as _;
use camino::Utf8PathBuf;
use itala_vault::{LocalFsVfs, Vault};

/// Open a vault and print its canonical path to stdout.
///
/// # Errors
/// Returns an error if the path does not exist or is not a valid vault root.
pub async fn open(path: Utf8PathBuf) -> anyhow::Result<()> {
    let std_canonical = tokio::fs::canonicalize(&path)
        .await
        .with_context(|| format!("failed to access vault at '{path}'"))?;
    let canonical = Utf8PathBuf::try_from(std_canonical)
        .map_err(|_| anyhow::anyhow!("vault path contains invalid UTF-8"))?;
    let vfs = LocalFsVfs::new(&canonical)
        .await
        .context("failed to initialize VFS")?;
    let vault = Vault::new(Arc::new(vfs), canonical.clone())
        .open()
        .await
        .context("failed to open vault")?;
    println!("opened vault {canonical}");
    let _ = vault.close();
    Ok(())
}
