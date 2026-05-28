//! Note command handlers.

use std::sync::Arc;

use anyhow::Context as _;
use camino::Utf8PathBuf;
use futures::StreamExt as _;
use itala_kernel::{CancellationToken, DynVfs, NoteId, VfsEvent};
use itala_vault::{Frontmatter, FrontmatterValue, LocalFsVfs, Vault};
use tokio::io::AsyncWriteExt as _;

/// List all notes sorted by `NoteId`, printing `<id>\t<path>` per line.
///
/// The FS location (path) is printed when available; a future non-FS backend
/// would emit `<id>` only.  Output is sorted by `NoteId` — the stable identifier.
///
/// # Errors
/// Returns an error if the vault cannot be opened or a VFS operation fails.
pub async fn list(vault_path: Utf8PathBuf) -> anyhow::Result<()> {
    let vfs = LocalFsVfs::new(&vault_path)
        .await
        .context("failed to initialize VFS")?;
    let v = Vault::new(Arc::new(vfs), vault_path)
        .open()
        .await
        .context("failed to open vault")?;
    // v.list() already returns ids sorted by NoteId.
    let ids = v.list().await?;
    let mut out = tokio::io::stdout();
    for id in ids {
        match v.fs_location_of(id).await? {
            Some(path) => out.write_all(format!("{id}\t{path}\n").as_bytes()).await?,
            None => out.write_all(format!("{id}\n").as_bytes()).await?,
        }
    }
    Ok(())
}

/// Print a note's YAML frontmatter (if any), a `---` separator, then the body verbatim.
///
/// # Errors
/// Returns an error if the note ID is invalid, the vault cannot be opened, or the
/// note is not found.
pub async fn read(vault_path: Utf8PathBuf, id_str: String) -> anyhow::Result<()> {
    let id: NoteId = id_str.parse().context("invalid note ID")?;
    let vfs = LocalFsVfs::new(&vault_path)
        .await
        .context("failed to initialize VFS")?;
    let v = Vault::new(Arc::new(vfs), vault_path)
        .open()
        .await
        .context("failed to open vault")?;
    let note = v.read(id).await?;
    let mut stdout = tokio::io::stdout();
    if !note.frontmatter.is_empty() {
        let yaml = render_frontmatter_yaml(&note.frontmatter);
        stdout.write_all(yaml.as_bytes()).await?;
        if !yaml.ends_with('\n') {
            stdout.write_all(b"\n").await?;
        }
    }
    stdout.write_all(b"---\n").await?;
    stdout.write_all(&note.body).await?;
    Ok(())
}

/// Watch the vault root for filesystem changes, printing one event line per change.
///
/// Blocks until SIGINT (Ctrl-C), then exits cleanly.
///
/// # Errors
/// Returns an error if the vault cannot be opened or a write to stdout fails.
pub async fn watch(vault_path: Utf8PathBuf) -> anyhow::Result<()> {
    let vfs: Arc<dyn DynVfs> = Arc::new(
        LocalFsVfs::new(&vault_path)
            .await
            .context("failed to initialize VFS")?,
    );
    let v = Vault::new(Arc::clone(&vfs), vault_path.clone())
        .open()
        .await
        .context("failed to open vault")?;

    let cancel = CancellationToken::new();
    let child_token = cancel.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        child_token.cancel();
    });

    let mut stream = vfs.watch(&vault_path);
    let mut stdout = tokio::io::stdout();
    loop {
        tokio::select! {
            biased;
            () = cancel.cancelled() => break,
            ev = stream.next() => {
                let Some(ev) = ev else { break };
                stdout
                    .write_all(format_event(&ev).as_bytes())
                    .await?;
                stdout.flush().await?;
            }
        }
    }
    let _ = v.close();
    Ok(())
}

// ---------------------------------------------------------------------------
// YAML emitter (hand-rolled, no serde_yaml / chrono dep)
// ---------------------------------------------------------------------------

#[must_use]
fn render_frontmatter_yaml(fm: &Frontmatter) -> String {
    let mut buf = String::new();
    for (key, val) in fm.iter() {
        emit_kv(key, val, 0, &mut buf);
    }
    buf
}

fn emit_kv(key: &str, val: &FrontmatterValue, indent: usize, buf: &mut String) {
    for _ in 0..indent {
        buf.push(' ');
    }
    buf.push_str(key);
    match val {
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => buf.push(':'),
        _ => buf.push_str(": "),
    }
    emit_val(val, indent, buf);
}

fn emit_val(val: &FrontmatterValue, indent: usize, buf: &mut String) {
    match val {
        serde_json::Value::Null => buf.push_str("null\n"),
        serde_json::Value::Bool(b) => {
            buf.push_str(if *b { "true" } else { "false" });
            buf.push('\n');
        }
        serde_json::Value::Number(n) => {
            buf.push_str(&n.to_string());
            buf.push('\n');
        }
        serde_json::Value::String(s) => {
            emit_scalar(s, buf);
            buf.push('\n');
        }
        serde_json::Value::Array(items) => {
            buf.push('\n');
            for item in items {
                for _ in 0..indent + 2 {
                    buf.push(' ');
                }
                buf.push_str("- ");
                match item {
                    serde_json::Value::String(s) => {
                        emit_scalar(s, buf);
                        buf.push('\n');
                    }
                    _ => emit_val(item, indent + 4, buf),
                }
            }
        }
        serde_json::Value::Object(map) => {
            buf.push('\n');
            for (k, v) in map {
                emit_kv(k, v, indent + 2, buf);
            }
        }
    }
}

/// Emit a scalar string, quoting if necessary to avoid YAML ambiguity.
fn emit_scalar(s: &str, buf: &mut String) {
    if needs_quoting(s) {
        buf.push('"');
        for c in s.chars() {
            match c {
                '"' => buf.push_str("\\\""),
                '\\' => buf.push_str("\\\\"),
                '\n' => buf.push_str("\\n"),
                '\r' => buf.push_str("\\r"),
                c => buf.push(c),
            }
        }
        buf.push('"');
    } else {
        buf.push_str(s);
    }
}

#[must_use]
fn needs_quoting(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    let first = s.chars().next().expect("non-empty");
    // YAML flow-indicator or block-indicator at start.
    if ":,[]{}#&*!|>'\"%@`-".contains(first) {
        return true;
    }
    // Contains ': ' which would parse as a mapping.
    if s.contains(": ") {
        return true;
    }
    // Looks like a YAML boolean or null.
    matches!(
        s,
        "true"
            | "false"
            | "null"
            | "yes"
            | "no"
            | "on"
            | "off"
            | "True"
            | "False"
            | "Null"
            | "Yes"
            | "No"
            | "On"
            | "Off"
            | "TRUE"
            | "FALSE"
            | "NULL"
    )
}

#[must_use]
fn format_event(ev: &VfsEvent) -> String {
    match ev {
        VfsEvent::Created { path } => format!("created\t{path}\n"),
        VfsEvent::Changed { path } => format!("changed\t{path}\n"),
        VfsEvent::Deleted { path } => format!("deleted\t{path}\n"),
        _ => String::new(),
    }
}
