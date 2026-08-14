//! Cross-cutting state threaded through every step: the loaded [`Config`],
//! the I/O [`Prompter`] + [`Style`], and the [`Exec`] implementation
//! (real or dry-run).

use crate::cli::Args;
use crate::config::Config;
use crate::io::{Prompter, Style};
use crate::process::{DryExec, Exec, RealExec};
use anyhow::{Context as _, Result};
use std::path::PathBuf;
use std::sync::Arc;

use super::Step;

pub struct Context {
    pub config: Config,
    pub prompter: Prompter,
    pub style: Style,
    pub exec: Arc<dyn Exec>,
    pub dry_run: bool,
    pub non_interactive: bool,
    /// Path the config was loaded from (kept for diagnostics + future
    /// `--write-back` support).
    #[allow(dead_code)]
    pub config_path: Option<PathBuf>,
}

impl Context {
    /// Build the initial context from CLI args.
    pub fn build(args: &Args) -> Result<Self> {
        let config =
            Config::load(args.config.as_deref()).context("failed to load configuration")?;
        let exec: Arc<dyn Exec> = if args.dry_run {
            Arc::new(DryExec)
        } else {
            Arc::new(RealExec)
        };
        Ok(Self {
            config,
            prompter: Prompter::new(args.non_interactive),
            style: Style,
            exec,
            dry_run: args.dry_run,
            non_interactive: args.non_interactive,
            config_path: args.config.clone(),
        })
    }

    /// Helper: run a shell command via the configured executor.
    pub fn run(&self, cmd: &str) -> Result<()> {
        self.exec.run(cmd).map(|_| ())
    }

    /// Helper: run multiple shell commands in order.
    pub fn run_all(&self, cmds: impl IntoIterator<Item = String>) -> Result<()> {
        for cmd in cmds {
            self.exec.run(&cmd)?;
        }
        Ok(())
    }

    /// Helper: write a file, but in `--dry-run` mode just print the
    /// intended contents to stdout instead of touching the filesystem.
    pub fn write_file(&self, path: &std::path::Path, contents: &str) -> Result<()> {
        if self.dry_run {
            println!("# >>> write {}", path.display());
            println!("{contents}");
            println!("# <<< end {}", path.display());
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, contents)?;
        Ok(())
    }
}

/// Catalog of every install step, in execution order. This is the Rust
/// equivalent of `InstallCommand::$steps`.
pub struct Steps;

impl Steps {
    pub fn ordered() -> Vec<Box<dyn Step>> {
        vec![
            Box::new(super::policies::PoliciesStep),
            Box::new(super::server::ServerSetupStep),
            Box::new(super::redis::RedisSetupStep),
            Box::new(super::prerequisites::PrerequisitesStep),
            Box::new(super::database::DatabaseStep),
            Box::new(super::php::PhpSetupStep),
            Box::new(super::nginx::NginxSetupStep),
            Box::new(super::unit3d::Unit3dSetupStep),
            Box::new(super::meilisearch::MeilisearchSetupStep),
            Box::new(super::credentials::CredentialsStep),
        ]
    }
}
