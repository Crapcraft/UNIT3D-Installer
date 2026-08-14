//! `require_root()` — equivalent of `IsPrivilegedUser` policy.

use anyhow::{Result, bail};
use nix::unistd::geteuid;

/// Returns `Ok(())` when the current process is running as UID 0.
pub fn require_root() -> Result<()> {
    if geteuid().as_raw() == 0 {
        Ok(())
    } else {
        bail!("Must be run as root (sudo) — current EUID is not 0")
    }
}
