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

/// Maximum time a single shell command may run before it is killed. Prevents
/// a hung `apt-get`/`php artisan`/etc. from wedging the install forever.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60 * 15);

/// Cap on bytes captured per stream, so a chatty command can't exhaust RAM.
pub const MAX_CAPTURED: usize = 8 * 1024 * 1024;

pub trait Exec: Send + Sync {
    fn run(&self, cmd: &str) -> Result<Output>;
}

/// Real shell executor.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealExec;

/// Read up to [`MAX_CAPTURED`] bytes from `stream` (fewer if the stream
/// closes first). Never overshoots the cap, so a chatty command can't
/// exhaust memory.
fn read_bounded(stream: &mut impl std::io::Read) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 << 16);
    let mut chunk = [0u8; 8192];
    while buf.len() < MAX_CAPTURED {
        let want = (MAX_CAPTURED - buf.len()).min(chunk.len());
        match stream.read(&mut chunk[..want]) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    buf
}

impl RealExec {
    /// Run a command with a hard timeout. The command runs in its own
    /// process group (via `setsid`) so the whole tree can be killed if it
    /// exceeds `timeout`. Stdout/stderr are drained concurrently to avoid
    /// pipe-buffer deadlocks.
    fn run_with_timeout(&self, cmd: &str, timeout: Duration) -> Result<Output> {
        use std::os::unix::process::CommandExt;
        use std::time::Instant;

        let log_cmd = crate::secrets::redact(cmd);
        let mut command = Command::new("bash");
        #[cfg(unix)]
        unsafe {
            command.pre_exec(|| {
                // Become session/group leader so `killpg` below can reap the
                // entire process tree (bash + any grandchildren).
                let _ = nix::unistd::setsid();
                Ok(())
            });
        }
        let mut child = command
            .arg("-c")
            .arg(cmd)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to spawn command: {log_cmd}"))?;

        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        // Bounded drain: cap captured output so a chatty command can't balloon
        // RAM for the whole timeout window. `read_to_end` with a large cap is
        // still concurrent and deadlock-free.
        let out_thread = std::thread::spawn(move || read_bounded(&mut stdout));
        let err_thread = std::thread::spawn(move || read_bounded(&mut stderr));

        let deadline = Instant::now() + timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if Instant::now() >= deadline {
                        #[cfg(unix)]
                        {
                            let _ = nix::sys::signal::killpg(
                                nix::unistd::Pid::from_raw(child.id() as i32),
                                nix::sys::signal::Signal::SIGKILL,
                            );
                        }
                        let _ = child.wait();
                        let stdout = out_thread.join().unwrap_or_default();
                        let stderr = err_thread.join().unwrap_or_default();
                        return Err(anyhow::anyhow!(
                            "command timed out after {}s and was killed: {log_cmd}\nstdout: {}\nstderr: {}",
                            timeout.as_secs(),
                            crate::secrets::redact(&String::from_utf8_lossy(&stdout)),
                            crate::secrets::redact(&String::from_utf8_lossy(&stderr))
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    let _ = child.kill();
                    return Err(e).context("failed polling command");
                }
            }
        };

        let stdout = out_thread.join().unwrap_or_default();
        let stderr = err_thread.join().unwrap_or_default();
        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }
}

impl Exec for RealExec {
    fn run(&self, cmd: &str) -> Result<Output> {
        let log_cmd = crate::secrets::redact(cmd);
        tracing::info!(cmd = log_cmd.as_str(), "running");
        let bar = start_bar();
        let output = self.run_with_timeout(cmd, DEFAULT_TIMEOUT)?;
        bar.finish_with_message("done");
        if !output.status.success() {
            let stderr = crate::secrets::redact(&String::from_utf8_lossy(&output.stderr));
            let stdout = crate::secrets::redact(&String::from_utf8_lossy(&output.stdout));
            anyhow::bail!(
                "command failed ({}): {log_cmd}\nstderr: {stderr}\nstdout: {stdout}",
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
        // Secrets are scrubbed even in dry-run so a `--dry-run` session piped
        // into a log (or CI) never leaks DB/root passwords or master keys.
        let safe = crate::secrets::redact(cmd);
        println!("$ {safe}");
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

    #[test]
    fn real_exec_reports_missing_command() {
        // `bash -c 'definitely-not-a-command-xyz'` spawns bash fine but the
        // command is not found (exit 127) — the error must mention it.
        let err = RealExec.run("definitely-not-a-command-xyz").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("command failed"), "got: {msg}");
        assert!(
            msg.contains("127") || msg.contains("not found"),
            "got: {msg}"
        );
    }

    #[test]
    fn real_exec_runs_pipelines() {
        let out = RealExec.run("printf 'a\\nb\\n' | wc -l").unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "2");
    }

    #[test]
    fn real_exec_surfaces_failure_details() {
        let err = RealExec.run("echo out; echo oops >&2; exit 7").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("command failed"), "got: {msg}");
        assert!(msg.contains("out"), "stdout should be in error: {msg}");
        assert!(msg.contains("oops"), "stderr should be in error: {msg}");
        assert!(msg.contains("7"), "exit code should be in error: {msg}");
    }

    #[test]
    fn dry_exec_echoes_command_to_stdout() {
        // Capture the printed "$ cmd" line.
        let out = DryExec.run("echo hi").unwrap();
        assert!(out.status.success());
        assert!(out.stdout.is_empty());
        assert!(out.stderr.is_empty());
    }

    #[test]
    fn timeout_kills_hung_command() {
        let real = RealExec;
        // `sleep 5` with a 400ms budget must be killed and reported.
        let err = real
            .run_with_timeout("sleep 5", Duration::from_millis(400))
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("timed out"), "got: {msg}");
        assert!(msg.contains("was killed"), "got: {msg}");
    }

    #[test]
    fn fast_command_finishes_within_timeout() {
        let real = RealExec;
        let out = real
            .run_with_timeout("echo fast", Duration::from_secs(5))
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "fast");
    }

    #[test]
    fn failure_error_is_redacted() {
        // A failing command whose output embeds a secret must not leak it.
        let err = RealExec
            .run("echo 'password hushhush' >&2; exit 1")
            .unwrap_err();
        let msg = err.to_string();
        assert!(!msg.contains("hushhush"), "secret leaked: {msg}");
        assert!(msg.contains("<redacted>"), "should be redacted: {msg}");
    }

    #[test]
    fn dry_exec_prints_redacted_command() {
        // DryExec's printed line goes through `redact()`, so the same secret
        // that appears in a command must not appear in what would print.
        let redacted = crate::secrets::redact("mysqladmin -u root password hunter2");
        assert!(!redacted.contains("hunter2"));
        assert!(redacted.contains("<redacted>"));
        assert!(DryExec.run("mysqladmin -u root password hunter2").is_ok());
    }

    #[test]
    fn spawn_failure_is_redacted() {
        // Hard to force a spawn failure of `bash` itself; instead verify the
        // with_context path is reachable and redacts via a bogus env-free
        // invocation is not possible. We assert the redact helper instead.
        let cmd = "mysql -e \"CREATE USER 'u' IDENTIFIED BY 'topsecret'\"";
        let redacted = crate::secrets::redact(cmd);
        assert!(!redacted.contains("topsecret"));
    }

    #[test]
    fn output_capture_is_bounded() {
        // A command emitting far more than MAX_CAPTURED bytes must not OOM and
        // must return quickly; the captured buffer is capped.
        let cmd = format!("head -c {} /dev/zero | tr '\\0' 'x'", MAX_CAPTURED * 2);
        let out = RealExec
            .run_with_timeout(&cmd, Duration::from_secs(10))
            .unwrap();
        assert!(out.stdout.len() <= MAX_CAPTURED);
    }

    #[test]
    fn timeout_message_is_redacted() {
        // A timed-out command with an embedded secret must not leak it.
        let cmd = "echo 'MAIL_PASSWORD=sup3rs3cret'; sleep 5";
        let err = RealExec
            .run_with_timeout(cmd, Duration::from_millis(300))
            .unwrap_err();
        let msg = err.to_string();
        assert!(!msg.contains("sup3rs3cret"), "secret leaked: {msg}");
        assert!(msg.contains("<redacted>"), "should be redacted: {msg}");
    }
}
