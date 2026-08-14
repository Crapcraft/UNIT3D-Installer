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
        .and_then(|s| s.split_whitespace().next().map(str::to_string))
        .unwrap_or_default()
}
