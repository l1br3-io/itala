//! Itala CLI shell (MVP-α).
//!
//! First user-facing surface per the roadmap. Uses `clap` derive for
//! subcommands and `tokio` for the runtime. Errors convert to user-facing
//! diagnostics at the boundary via `anyhow` (ARCHITECTURE.md §7.1).
//!
//! M1 surface:
//! - `init`                       — initialize a vault
//! - `notes {create,list,show,delete}`
//! - `history {list,show,restore}` (added in F10)
//! - `notes move`                 (added in F12)

#![forbid(unsafe_code)]

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};

use itala_kernel::vfs::into_dyn;
use itala_kernel::vfs_types::VfsPath;
use itala_vault::Vault;

/// Itala — a local-first, plugin-extensible knowledge base.
#[derive(Debug, Parser)]
#[command(name = "itala", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize a new vault at the given path (default: current directory).
    Init {
        /// Vault root directory. Created if missing.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Note operations.
    Notes {
        #[command(subcommand)]
        action: NotesAction,
    },
    /// Version history operations.
    History {
        #[command(subcommand)]
        action: HistoryAction,
    },
    /// Print the version and exit.
    Version,
}

#[derive(Debug, Subcommand)]
enum NotesAction {
    /// Create a new note with the given title.
    Create {
        /// Note title (case-preserved; slugified into the filename).
        title: String,
        /// Vault root directory. Default: current directory.
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// Body text. If omitted, reads from stdin.
        #[arg(long)]
        body: Option<String>,
    },
    /// List notes in the vault.
    List {
        /// Vault root directory. Default: current directory.
        #[arg(long, default_value = ".")]
        vault: PathBuf,
    },
    /// Print a note's contents to stdout.
    Show {
        /// Vault root directory. Default: current directory.
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// Path of the note relative to the vault root (e.g. `Notes/x-<ulid>.md`).
        path: String,
    },
    /// Delete a note.
    Delete {
        /// Vault root directory. Default: current directory.
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// Path of the note relative to the vault root.
        path: String,
        /// Skip the confirmation prompt.
        #[arg(long)]
        yes: bool,
    },
    /// Move (rename) a note, rewriting inbound `[[wiki-links]]`.
    Move {
        /// Vault root directory. Default: current directory.
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// Path of the note to move.
        from: String,
        /// New path (must include the `.md` suffix).
        to: String,
    },
}

#[derive(Debug, Subcommand)]
enum HistoryAction {
    /// List snapshots for a note.
    List {
        /// Vault root directory. Default: current directory.
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// Note path (e.g. `Notes/x-<ulid>.md`) or ULID.
        note: String,
    },
    /// Print a snapshot's body to stdout.
    Show {
        /// Vault root directory. Default: current directory.
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// History snapshot identifier (ULID).
        id: String,
    },
    /// Restore a snapshot by writing it back as a new save.
    Restore {
        /// Vault root directory. Default: current directory.
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// History snapshot identifier (ULID).
        id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Version => {
            println!("itala {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Commands::Init { path } => {
            let abs = canonicalize_root(&path)?;
            let vfs = open_vfs(&abs)?;
            let vault = Vault::init(vfs)
                .await
                .with_context(|| format!("initializing vault at {abs}"))?;
            println!("initialized vault {} at {}", vault.id(), abs);
            Ok(())
        }
        Commands::Notes { action } => handle_notes(action).await,
        Commands::History { action } => handle_history(action).await,
    }
}

async fn handle_history(action: HistoryAction) -> Result<()> {
    match action {
        HistoryAction::List { vault, note } => {
            let v = open_vault_with_history(&vault).await?;
            let note_id = resolve_note_id(&v, &note).await?;
            let entries = v.history(note_id).await.context("history lookup failed")?;
            if entries.is_empty() {
                println!("(no history)");
                return Ok(());
            }
            for entry in entries {
                println!("{}\t{}", entry.id, entry.timestamp);
            }
            Ok(())
        }
        HistoryAction::Show { vault, id } => {
            let v = open_vault_with_history(&vault).await?;
            let history_id: itala_kernel::HistoryId =
                id.parse().map_err(|e| anyhow!("invalid history id: {e}"))?;
            let bytes = v
                .restore_history(history_id)
                .await
                .with_context(|| format!("restore {id}"))?;
            let stdout = std::io::stdout();
            let mut lock = stdout.lock();
            lock.write_all(&bytes).context("write stdout")?;
            Ok(())
        }
        HistoryAction::Restore { vault, id } => {
            let v = open_vault_with_history(&vault).await?;
            let history_id: itala_kernel::HistoryId =
                id.parse().map_err(|e| anyhow!("invalid history id: {e}"))?;
            let bytes = v
                .restore_history(history_id)
                .await
                .with_context(|| format!("restore {id}"))?;
            let entry = v
                .get_history(history_id)
                .await
                .with_context(|| format!("get {id}"))?;
            // Re-save as a new snapshot via the vault.
            let note_id = entry.note_id;
            let note_path = find_note_path_by_id(&v, note_id)
                .await
                .ok_or_else(|| anyhow!("note {note_id} not found in vault"))?;
            let mut note = v
                .read_note(&note_path)
                .await
                .with_context(|| format!("read {note_path}"))?;
            // Parse body from the restored bytes (strip frontmatter if present).
            let restored = itala_note::Note::from_markdown(
                note.id,
                note.path.clone(),
                bytes,
                note.created_at,
                time::OffsetDateTime::now_utc(),
            )
            .map_err(|e| anyhow!("parse restored: {e:?}"))?;
            note.body = restored.body;
            note.frontmatter = restored.frontmatter;
            v.save_note(&note).await.context("save_note failed")?;
            println!("restored {} as a new snapshot on {}", history_id, note.path);
            Ok(())
        }
    }
}

/// Resolves a `note` argument that may be either a ULID or a vault-relative
/// path to a [`itala_kernel::NoteId`].
async fn resolve_note_id(v: &Vault, note: &str) -> Result<itala_kernel::NoteId> {
    // Try ULID first.
    if let Ok(id) = note.parse::<itala_kernel::NoteId>() {
        return Ok(id);
    }
    // Otherwise treat as a path.
    let p = VfsPath::new(note).context("invalid path")?;
    let n = v
        .read_note(&p)
        .await
        .with_context(|| format!("read {note}"))?;
    Ok(n.id)
}

/// Scans the vault for the note whose id matches; returns its path.
async fn find_note_path_by_id(v: &Vault, id: itala_kernel::NoteId) -> Option<VfsPath> {
    let notes = v.list_notes().await.ok()?;
    notes.into_iter().find(|n| n.id == id).map(|n| n.path)
}

async fn handle_notes(action: NotesAction) -> Result<()> {
    match action {
        NotesAction::Create { title, vault, body } => {
            let v = open_vault_with_history(&vault).await?;
            let body_text = match body {
                Some(s) => s,
                None => read_stdin_to_string().await?,
            };
            let created = v
                .create_note(&title, &body_text)
                .await
                .context("create_note failed")?;
            println!("created: {} (id: {})", created.note.path, created.note.id);
            Ok(())
        }
        NotesAction::List { vault } => {
            let v = open_vault_with_history(&vault).await?;
            let notes = v.list_notes().await.context("list_notes failed")?;
            if notes.is_empty() {
                println!("(no notes)");
                return Ok(());
            }
            for meta in notes {
                println!("{}\t{}", meta.id, meta.path);
            }
            Ok(())
        }
        NotesAction::Show { vault, path } => {
            let v = open_vault_with_history(&vault).await?;
            let p = VfsPath::new(&path).context("invalid path")?;
            let note = v
                .read_note(&p)
                .await
                .with_context(|| format!("read {path}"))?;
            let bytes = note.to_markdown();
            let stdout = std::io::stdout();
            let mut lock = stdout.lock();
            lock.write_all(&bytes).context("write stdout")?;
            Ok(())
        }
        NotesAction::Delete { vault, path, yes } => {
            if !yes {
                return Err(anyhow!(
                    "refusing to delete without --yes (non-interactive mode)"
                ));
            }
            let v = open_vault_with_history(&vault).await?;
            let p = VfsPath::new(&path).context("invalid path")?;
            v.delete_note(&p)
                .await
                .with_context(|| format!("delete {path}"))?;
            println!("deleted: {path}");
            Ok(())
        }
        NotesAction::Move { vault, from, to } => {
            let v = open_vault_with_history(&vault).await?;
            let from_p = VfsPath::new(&from).context("invalid from path")?;
            let to_p = VfsPath::new(&to).context("invalid to path")?;
            let report = v
                .move_note(&from_p, &to_p)
                .await
                .with_context(|| format!("move {from} → {to}"))?;
            println!(
                "moved: {} → {} (rewrote {} note(s))",
                report.moved_from,
                report.moved_to,
                report.rewritten.len()
            );
            for path in &report.rewritten {
                println!("\trewrote: {path}");
            }
            Ok(())
        }
    }
}

/// Canonicalizes a user-supplied path into an absolute UTF-8 path, creating
/// the directory if missing.
fn canonicalize_root(p: &Path) -> Result<Utf8PathBuf> {
    if !p.exists() {
        std::fs::create_dir_all(p).with_context(|| format!("mkdir {}", p.display()))?;
    }
    let canon =
        std::fs::canonicalize(p).with_context(|| format!("canonicalize {}", p.display()))?;
    Utf8PathBuf::from_path_buf(canon).map_err(|e| anyhow!("vault root not UTF-8: {}", e.display()))
}

/// Builds a `LocalFsVfs` boxed as `dyn DynVfs` over the given root.
fn open_vfs(root: &camino::Utf8Path) -> Result<Box<dyn itala_kernel::vfs::DynVfs>> {
    let adapter = itala_vfs_fs::LocalFsVfs::new(root).context("open vfs")?;
    Ok(into_dyn(adapter))
}

/// Opens an existing vault at `root`. Without history wiring; kept for
/// callers that want to inspect a vault without snapshotting.
#[allow(dead_code)]
async fn open_vault(root: &Path) -> Result<Box<Vault>> {
    let abs = canonicalize_root(root)?;
    let vfs = open_vfs(&abs)?;
    let v = Vault::open(vfs)
        .await
        .with_context(|| format!("opening vault at {abs}"))?;
    Ok(Box::new(v))
}

/// Opens a vault and attaches a [`itala_store_files::FilesHistoryStore`] over
/// its VFS.
async fn open_vault_with_history(root: &Path) -> Result<Box<Vault>> {
    use std::sync::OnceLock;
    // The history adapter needs its own VFS handle to the same root, since
    // `Vault` consumes the Box<dyn DynVfs>. We construct a second LocalFsVfs
    // pointed at the same directory.
    let abs = canonicalize_root(root)?;
    let vfs_for_vault = open_vfs(&abs)?;
    let mut v = Vault::open(vfs_for_vault)
        .await
        .with_context(|| format!("opening vault at {abs}"))?;
    let vfs_for_history = open_vfs(&abs)?;
    let history: Box<dyn itala_kernel::history::DynHistoryStore> =
        Box::new(itala_store_files::FilesHistoryStore::new(vfs_for_history));
    v.set_history(history);
    let _ = OnceLock::<()>::new(); // anchor for future global state
    Ok(Box::new(v))
}

async fn read_stdin_to_string() -> Result<String> {
    use tokio::io::AsyncReadExt;
    let mut buf = String::new();
    tokio::io::stdin().read_to_string(&mut buf).await?;
    Ok(buf)
}
