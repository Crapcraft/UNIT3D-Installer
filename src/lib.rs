//! UNIT3D-Community-Edition installer (Rust port) — library crate.
//!
//! Exposes the modules so integration tests (in `tests/`) can drive the
//! step pipeline with a mocked [`Exec`]. The binary `main.rs` is a thin
//! wrapper around [`run`].

pub mod cli;
pub mod config;
pub mod credentials;
pub mod io;
pub mod password;
pub mod process;
pub mod resources;
pub mod steps;
pub mod system;

pub use crate::process::Exec;

use anyhow::Result;
use clap::Parser;

/// Entrypoint shared by the `main` binary.
pub fn run() -> Result<()> {
    let args = cli::Args::parse();
    init_tracing(args.verbosity);

    print_intro();

    let mut ctx = steps::Context::build(&args)?;
    let runner = steps::StepRunner;
    runner.run(&mut ctx)?;

    ctx.style.final_summary(&ctx.config);
    Ok(())
}

/// Print the ASCII banner.
fn print_intro() {
    use crate::resources::intro::IntroTemplate;
    use askama::Template;
    let tpl = IntroTemplate;
    println!("{}", tpl.render().unwrap_or_default());
}

/// Initialize `tracing` based on the CLI verbosity.
fn init_tracing(verbosity: u8) {
    let filter = match verbosity {
        0 => "unit3d_installer=warn",
        1 => "unit3d_installer=info",
        2 => "unit3d_installer=debug",
        _ => "unit3d_installer=trace,debug",
    };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}

#[cfg(test)]
mod tests {
    #[test]
    fn cli_parses_minimal() {
        use crate::cli::Args;
        use clap::Parser;
        let args = Args::parse_from(["unit3d-installer", "--non-interactive"]);
        assert!(args.non_interactive);
        assert!(!args.dry_run);
        assert!(args.config.is_none());
    }
}
