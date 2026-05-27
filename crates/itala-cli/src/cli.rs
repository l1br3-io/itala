//! `clap`-derived argument types for the `itala` CLI.

use camino::Utf8PathBuf;

/// Top-level CLI entry point.
#[derive(clap::Parser)]
#[command(name = "itala", version, about = "Itala vault smoke CLI")]
pub struct Cli {
    /// Subcommand to run.
    #[command(subcommand)]
    pub cmd: Command,
}

/// Top-level subcommands.
#[derive(clap::Subcommand)]
pub enum Command {
    /// Vault management commands.
    Vault {
        /// Vault subcommand.
        #[command(subcommand)]
        cmd: VaultCmd,
    },
    /// Note management commands.
    Note {
        /// Note subcommand.
        #[command(subcommand)]
        cmd: NoteCmd,
    },
}

/// Subcommands under `itala vault`.
#[derive(clap::Subcommand)]
pub enum VaultCmd {
    /// Open a vault at the given path and print its canonical path.
    Open {
        /// Path to the vault root directory.
        path: Utf8PathBuf,
    },
}

/// Subcommands under `itala note`.
#[derive(clap::Subcommand)]
pub enum NoteCmd {
    /// List all notes in the vault, sorted by path.
    List {
        /// Path to the vault root directory.
        #[arg(long)]
        vault: Utf8PathBuf,
    },
    /// Print the frontmatter and body of a note.
    Read {
        /// Path to the vault root directory.
        #[arg(long)]
        vault: Utf8PathBuf,
        /// Note ID (e.g. `note_01ARZ3NDEKTSV4RRFFQ69G5FAV`).
        id: String,
    },
    /// Watch the vault for filesystem changes, printing one line per event.
    Watch {
        /// Path to the vault root directory.
        #[arg(long)]
        vault: Utf8PathBuf,
    },
}
