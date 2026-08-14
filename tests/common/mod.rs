//! Shared test helpers: an in-memory [`Exec`] that records commands instead
//! of running them, plus a `Context` builder for step-level tests.

use std::sync::{Arc, Mutex};
use unit3d_installer::process::Exec;
use unit3d_installer::steps::Context;

/// Records every command sent through the installer's executor without
/// actually running anything. This is how the step tests assert that the
/// right shell commands are emitted.
#[derive(Default, Clone)]
pub struct MockExec {
    pub commands: Arc<Mutex<Vec<String>>>,
}

impl MockExec {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ran(&self) -> Vec<String> {
        self.commands.lock().unwrap().clone()
    }

    pub fn any(&self, needle: &str) -> bool {
        self.ran().iter().any(|c| c.contains(needle))
    }
}

impl Exec for MockExec {
    fn run(&self, cmd: &str) -> Result<std::process::Output, anyhow::Error> {
        self.commands.lock().unwrap().push(cmd.to_string());
        Ok(std::process::Output {
            status: success_status(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }
}

use std::os::unix::process::ExitStatusExt;
fn success_status() -> std::process::ExitStatus {
    ExitStatusExt::from_raw(0)
}

/// Build a non-interactive, dry-run-free `Context` wired to a [`MockExec`].
/// Tests then call `ctx.exec.ran()` to inspect emitted commands.
pub fn test_context() -> (Context, MockExec) {
    let exec = MockExec::new();
    let ctx = Context {
        config: unit3d_installer::config::Config::default(),
        prompter: unit3d_installer::io::Prompter::new(true),
        style: unit3d_installer::io::Style,
        exec: Arc::new(exec.clone()),
        dry_run: false,
        non_interactive: true,
        config_path: None,
    };
    (ctx, exec)
}

/// Like [`test_context`] but with `dry_run = true`, so file-writes are
/// printed instead of touching the filesystem. Steps that shell out can
/// still be inspected via the recorded [`MockExec`].
#[allow(dead_code)] // only used by some test binaries
pub fn test_context_dry() -> (Context, MockExec) {
    let (mut ctx, exec) = test_context();
    ctx.dry_run = true;
    (ctx, exec)
}
