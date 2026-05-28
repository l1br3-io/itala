//! Itala vault smoke CLI.

use clap::Parser as _;

mod cli;
mod commands;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("itala: {e}");
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    match cli.cmd {
        cli::Command::Vault { cmd } => match cmd {
            cli::VaultCmd::Open { path } => commands::vault::open(path).await,
        },
        cli::Command::Note { cmd } => match cmd {
            cli::NoteCmd::List { vault } => commands::note::list(vault).await,
            cli::NoteCmd::Read { vault, id } => commands::note::read(vault, id).await,
            cli::NoteCmd::Watch { vault } => commands::note::watch(vault).await,
        },
    }
}
