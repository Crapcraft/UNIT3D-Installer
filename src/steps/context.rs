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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use clap::Parser;
    use tempfile::tempdir;

    #[test]
    fn build_selects_dry_exec_when_dry_run() {
        let args = Args::parse_from(["unit3d-installer", "--dry-run", "--non-interactive"]);
        let ctx = Context::build(&args).unwrap();
        assert!(ctx.dry_run);
        assert!(ctx.non_interactive);
        // DryExec must succeed without touching the system.
        ctx.run("echo hello").unwrap();
        ctx.run_all(["echo a".to_string(), "echo b".to_string()])
            .unwrap();
    }

    #[test]
    fn build_uses_real_exec_by_default() {
        let args = Args::parse_from(["unit3d-installer", "--non-interactive"]);
        let ctx = Context::build(&args).unwrap();
        assert!(!ctx.dry_run);
        // RealExec runs `true` fine.
        ctx.run("true").unwrap();
    }

    #[test]
    fn write_file_dry_run_does_not_touch_disk() {
        let args = Args::parse_from(["unit3d-installer", "--dry-run"]);
        let ctx = Context::build(&args).unwrap();
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("nested/deep/out.txt");
        ctx.write_file(&target, "contents").unwrap();
        // Parent directories must NOT have been created in dry-run mode.
        assert!(!tmp.path().join("nested").exists());
    }

    #[test]
    fn write_file_creates_parent_dirs() {
        let args = Args::parse_from(["unit3d-installer", "--non-interactive"]);
        let ctx = Context::build(&args).unwrap();
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("a/b/c.txt");
        ctx.write_file(&target, "hello").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "hello");
    }

    #[test]
    fn write_file_overwrites_existing() {
        let args = Args::parse_from(["unit3d-installer", "--non-interactive"]);
        let ctx = Context::build(&args).unwrap();
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("f.txt");
        std::fs::write(&target, "old").unwrap();
        ctx.write_file(&target, "new").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
    }

    #[test]
    fn run_all_short_circuits_on_failure() {
        struct Boom;
        impl Exec for Boom {
            fn run(&self, cmd: &str) -> Result<std::process::Output> {
                if cmd == "fail" {
                    anyhow::bail!("boom");
                }
                Ok(std::process::Output {
                    status: std::os::unix::process::ExitStatusExt::from_raw(0),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            }
        }
        let ctx = Context {
            config: Config::default(),
            prompter: Prompter::new(true),
            style: Style,
            exec: Arc::new(Boom),
            dry_run: false,
            non_interactive: true,
            config_path: None,
        };
        let res = ctx.run_all(["ok".to_string(), "fail".to_string(), "never".to_string()]);
        assert!(res.is_err());
    }

    #[test]
    fn config_path_is_forwarded() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg_path = tmp.path().join("cfg.toml");
        std::fs::write(&cfg_path, "[app]\nhostname = \"x.com\"\n").unwrap();
        let args = Args::parse_from([
            "unit3d-installer",
            "--config",
            cfg_path.to_str().unwrap(),
            "--dry-run",
        ]);
        let ctx = Context::build(&args).unwrap();
        assert_eq!(ctx.config_path.as_deref(), Some(cfg_path.as_path()));
        assert_eq!(ctx.config.app.hostname, "x.com");
    }
}
