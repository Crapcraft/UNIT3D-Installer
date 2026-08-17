//! Network facts: `hostname`, `fqdn`, primary `ip` — mirrors the legacy
//! `helpers.php` functions.

use std::process::Command;

pub fn hostname() -> String {
    Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

pub fn fqdn() -> String {
    Command::new("hostname")
        .arg("-f")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(hostname)
}

pub fn ip() -> String {
    // `hostname -I` returns all IPs space-separated; take the first.
    Command::new("hostname")
        .arg("-I")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| first_token(&s))
        .unwrap_or_default()
}

/// First whitespace-separated token of a command's stdout (used for `hostname
/// -I`). Empty/whitespace-only input yields `None`.
fn first_token(s: &str) -> Option<String> {
    s.split_whitespace().next().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostname_never_panics() {
        // Returns the machine hostname or empty string — must not panic.
        let _ = hostname();
    }

    #[test]
    fn fqdn_never_panics() {
        let _ = fqdn();
    }

    #[test]
    fn ip_never_panics() {
        let _ = ip();
    }

    #[test]
    fn hostname_falls_back_to_empty_on_garbage_command() {
        // Simulate a failure: run the underlying logic with a bogus binary.
        let out = Command::new("definitely-not-a-command-xyz")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        assert_eq!(out, "");
    }

    #[test]
    fn first_token_takes_leading_ip() {
        // `hostname -I` output: all IPs space-separated, newline at end.
        assert_eq!(
            first_token("192.168.1.10 10.0.0.1\n").unwrap(),
            "192.168.1.10"
        );
        assert_eq!(first_token("  10.0.0.5\n").unwrap(), "10.0.0.5");
        assert_eq!(first_token("10.0.0.5").unwrap(), "10.0.0.5");
    }

    #[test]
    fn first_token_handles_empty_and_whitespace() {
        assert!(first_token("").is_none());
        assert!(first_token("   \n\t ").is_none());
        // Trailing newline alone is not an IP.
        assert!(first_token("\n").is_none());
    }
}
