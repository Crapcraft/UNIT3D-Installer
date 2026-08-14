//! Command-line interface definition (clap v4 derive).

use clap::Parser;
use std::path::PathBuf;

/// UNIT3D-Community-Edition installer.
///
/// Provisions a fresh Ubuntu server with the UNIT3D platform, including
/// PHP-FPM, MariaDB/MySQL/PostgreSQL, Redis, Nginx, Supervisor, Meilisearch
/// and Laravel Echo Server.
#[derive(Parser, Debug, Clone)]
#[command(name = "unit3d-installer", version, about, long_about = None)]
pub struct Args {
    /// Path to a TOML configuration file.
    ///
    /// When provided, prompts are skipped for any field already present in
    /// the file. Fields still missing are prompted interactively unless
    /// `--non-interactive` is set (in which case defaults are used).
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Skip every interactive prompt. Requires a complete configuration
    /// file via `--config` and is intended for unattended deployments.
    #[arg(long, alias = "yes-to-all")]
    pub non_interactive: bool,

    /// Dry-run: render every step's commands and stub contents to stdout
    /// without touching the system. Useful for inspecting the plan.
    #[arg(long)]
    pub dry_run: bool,

    /// Increase logging verbosity (-v info, -vv debug, -vvv trace).
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbosity: u8,
}
