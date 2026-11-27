//! `larpidarium` command-line entry point.

mod build;
mod check;
mod edit;
mod serve;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use miette::{Result, miette};

/// Turn a LARP written in TOML into a graph web page and character sheets.
#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate a game folder and report errors.
    Check {
        /// Folder containing `game.toml`.
        dir: PathBuf,
    },
    /// Print the JSON Schema of the game format.
    Schema,
    /// Generate the static site into `dist/`.
    Build {
        /// Folder containing `game.toml`.
        dir: PathBuf,
        /// Output folder.
        #[arg(short, long, default_value = "dist")]
        out: PathBuf,
        /// Open the generated page in the default browser.
        #[arg(long)]
        open: bool,
    },
    /// Serve the game on <http://localhost> with editing in the browser and live reload.
    Serve {
        /// Folder containing `game.toml`.
        dir: PathBuf,
        /// Port to listen on (only on 127.0.0.1).
        #[arg(short, long, default_value_t = 7878)]
        port: u16,
        /// Open the page in the default browser.
        #[arg(long)]
        open: bool,
    },
    /// Convert a taraxaplan JSON save file into a game folder.
    Import {
        /// The taraxaplan `.json` file.
        file: PathBuf,
    },
    /// Generate player and organizer PDFs.
    Pdf {
        /// Folder containing `game.toml`.
        dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (name, phase) = match cli.command {
        Command::Check { dir } => return check::run(&dir),
        Command::Schema => ("schema", 1),
        Command::Build { dir, out, open } => return build::run(&dir, &out, open),
        Command::Serve { dir, port, open } => return serve::run(&dir, port, open),
        Command::Import { .. } => ("import", 2),
        Command::Pdf { .. } => ("pdf", 4),
    };
    Err(miette!(
        "`{name}` is not implemented yet (planned for phase {phase})"
    ))
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
