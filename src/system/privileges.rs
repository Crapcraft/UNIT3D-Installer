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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_requirement_reflects_current_euid() {
        // Whichever way it resolves, the helper must not panic and must be
        // consistent with geteuid.
        let euid = geteuid().as_raw();
        let result = require_root();
        if euid == 0 {
            assert!(result.is_ok());
        } else {
            let err = result.unwrap_err();
            assert!(err.to_string().contains("root"));
            assert!(err.to_string().contains("sudo"));
        }
    }
}
