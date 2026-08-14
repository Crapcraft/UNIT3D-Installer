//! Subprocess execution wrapper — Rust equivalent of the legacy
//! `src/Classes/Process.php` (Symfony Process + ProgressBar).
//!
//! Two implementations live behind one trait:
//!
//! * [`RealExec`] — actually shells out via `std::process::Command`.
//! * [`DryExec`]  — emits the command to stdout without touching the system
//!   (used by `--dry-run` and unit tests).
//!
//! Tests inject [`MockExec`] from the `tests/` directory.

use anyhow::{Context as _, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

pub trait Exec: Send + Sync {
    fn run(&self, cmd: &str) -> Result<Output>;
}

/// Real shell executor.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealExec;

impl Exec for RealExec {
    fn run(&self, cmd: &str) -> Result<Output> {
        tracing::info!(cmd, "running");
        let bar = start_bar();
        let output = Command::new("bash")
            .arg("-c")
            .arg(cmd)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .with_context(|| format!("failed to spawn command: {cmd}"))?;
        bar.finish_with_message("done");
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "command failed ({}): {cmd}\nstderr: {stderr}\nstdout: {stdout}",
                output.status
            );
        }
        Ok(output)
    }
}

/// Dry-run executor used by `--dry-run`. Prints commands instead of running.
#[derive(Debug, Clone, Copy, Default)]
pub struct DryExec;

impl Exec for DryExec {
    fn run(&self, cmd: &str) -> Result<Output> {
        println!("$ {cmd}");
        Ok(Output {
            status: success_status(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }
}

#[cfg(unix)]
fn success_status() -> std::process::ExitStatus {
    ExitStatusExt::from_raw(0)
}

#[cfg(not(unix))]
fn success_status() -> std::process::ExitStatus {
    // Fallback for non-Unix: run `true` (cmd /c ver) to obtain a real
    // success ExitStatus. The installer only supports Ubuntu, so this path
    // is only taken under strange test conditions.
    Command::new("true").status().unwrap()
}

fn start_bar() -> ProgressBar {
    let bar = ProgressBar::new_spinner();
    bar.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] {spinner} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    bar.enable_steady_tick(Duration::from_millis(120));
    bar.set_message("please wait...");
    bar
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_exec_runs_echo() {
        let out = RealExec.run("echo hello").unwrap();
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "hello");
    }

    #[test]
    fn real_exec_captures_stderr() {
        let err = RealExec.run("echo boom >&2; exit 3").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("boom"), "stderr should surface: {msg}");
        assert!(
            msg.contains("command failed"),
            "should mention failure: {msg}"
        );
    }

    #[test]
    fn real_exec_reports_failure() {
        let err = RealExec.run("exit 42").unwrap_err();
        assert!(err.to_string().contains("42"));
    }

    #[test]
    fn dry_exec_succeeds_and_records_nothing() {
        let out = DryExec.run("echo anything").unwrap();
        assert!(out.status.success());
        assert!(out.stdout.is_empty());
        assert!(out.stderr.is_empty());
    }

    #[test]
    fn success_status_helper() {
        let s = success_status();
        assert!(s.success());
    }
}
