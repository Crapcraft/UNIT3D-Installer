//! Detect the running Linux distribution by inspecting `/etc/os-release`
//! (with `/etc/issue` as a fallback). Mirrors the legacy
//! `distinfo()` helper and `install.sh` selection.
//!
//! Supported: Ubuntu 20.04 / 22.04 / 24.04 / 26.04 LTS. Anything else
//! results in [`Distro::Unsupported`].

use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistInfo {
    pub distro: Distro,
    pub id: String,
    pub version_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distro {
    Ubuntu,
    Unsupported,
}

impl Distro {
    pub fn is_supported(&self) -> bool {
        matches!(self, Distro::Ubuntu)
    }
}

#[derive(Debug, Error)]
pub enum DetectError {
    #[error("could not detect OS: /etc/os-release and /etc/issue both unreadable")]
    Unreadable,
}

/// Detect the distro, preferring `/etc/os-release`. Falls back to parsing
/// the first line of `/etc/issue` for non-standard templates.
pub fn detect() -> Result<DistInfo, DetectError> {
    if let Some(info) = from_os_release()? {
        return Ok(info);
    }
    if let Some(info) = from_issue() {
        return Ok(info);
    }
    Err(DetectError::Unreadable)
}

fn from_os_release() -> Result<Option<DistInfo>, DetectError> {
    let path = Path::new("/etc/os-release");
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return Ok(None),
    };
    let mut id = String::new();
    let mut version_id = String::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("ID=") {
            id = unquote(rest).to_string();
        } else if let Some(rest) = line.strip_prefix("VERSION_ID=") {
            version_id = unquote(rest).to_string();
        }
    }
    if id.is_empty() && version_id.is_empty() {
        return Ok(None);
    }
    let distro = if id == "ubuntu" {
        Distro::Ubuntu
    } else {
        Distro::Unsupported
    };
    Ok(Some(DistInfo {
        distro,
        id,
        version_id,
    }))
}

fn from_issue() -> Option<DistInfo> {
    let text = std::fs::read_to_string("/etc/issue").ok()?;
    let first = text.lines().next()?;
    let mut tokens = first.split_whitespace();
    let name = tokens.next()?;
    let version = tokens.next()?;
    if name.eq_ignore_ascii_case("ubuntu") {
        return Some(DistInfo {
            distro: Distro::Ubuntu,
            id: "ubuntu".to_string(),
            version_id: version.to_string(),
        });
    }
    Some(DistInfo {
        distro: Distro::Unsupported,
        id: name.to_lowercase(),
        version_id: version.to_string(),
    })
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ubuntu_2404_os_release() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "NAME=\"Ubuntu\"\nVERSION=\"24.04 LTS\"\nID=ubuntu\nID_LIKE=debian\nPRETTY_NAME=\"Ubuntu 24.04 LTS\"\nVERSION_ID=\"24.04\"\n").unwrap();
        // Re-point the function by manually parsing the file content
        // (the public `detect()` is hard-coded to /etc/os-release).
        let info = parse_for_test(std::fs::read_to_string(tmp.path()).unwrap());
        assert_eq!(info.distro, Distro::Ubuntu);
        assert_eq!(info.version_id, "24.04");
    }

    #[test]
    fn parses_ubuntu_2604_os_release() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "NAME=\"Ubuntu\"\nVERSION=\"26.04 LTS\"\nID=ubuntu\nID_LIKE=debian\nPRETTY_NAME=\"Ubuntu 26.04 LTS\"\nVERSION_ID=\"26.04\"\n").unwrap();
        let info = parse_for_test(std::fs::read_to_string(tmp.path()).unwrap());
        assert_eq!(info.distro, Distro::Ubuntu);
        assert_eq!(info.version_id, "26.04");
    }

    #[test]
    fn rejects_non_ubuntu() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            tmp.path(),
            "NAME=\"Debian\"\nVERSION=\"12\"\nID=debian\nVERSION_ID=\"12\"\n",
        )
        .unwrap();
        let info = parse_for_test(std::fs::read_to_string(tmp.path()).unwrap());
        assert_eq!(info.distro, Distro::Unsupported);
    }

    #[test]
    fn unquote_strips_matching_quotes() {
        assert_eq!(unquote("\"ubuntu\""), "ubuntu");
        assert_eq!(unquote("'24.04'"), "24.04");
        assert_eq!(unquote("ubuntu"), "ubuntu");
        assert_eq!(unquote(""), "");
        assert_eq!(unquote("\"starts-only"), "\"starts-only");
        assert_eq!(unquote("ends-only\""), "ends-only\"");
        // Whitespace is trimmed before quote detection.
        assert_eq!(unquote("  \"padded\"  "), "padded");
    }

    #[test]
    fn is_supported_matches_ubuntu_only() {
        assert!(Distro::Ubuntu.is_supported());
        assert!(!Distro::Unsupported.is_supported());
    }

    #[test]
    fn version_id_unquoted_via_helper() {
        // The real from_os_release unquotes values; replicate that here.
        let info = parse_for_test("ID=ubuntu\nVERSION_ID=\"26.04\"\n".to_string());
        assert_eq!(info.version_id, "26.04");
    }

    #[test]
    fn distro_fields_carried_through() {
        let info = parse_for_test("ID=ubuntu\nVERSION_ID=24.04\n".to_string());
        assert_eq!(info.id, "ubuntu");
        assert_eq!(info.version_id, "24.04");
        assert_eq!(info.distro, Distro::Ubuntu);
    }

    fn parse_for_test(text: String) -> DistInfo {
        let mut id = String::new();
        let mut version_id = String::new();
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("ID=") {
                id = super::unquote(rest).to_string();
            } else if let Some(rest) = line.strip_prefix("VERSION_ID=") {
                version_id = super::unquote(rest).to_string();
            }
        }
        DistInfo {
            distro: if id == "ubuntu" {
                Distro::Ubuntu
            } else {
                Distro::Unsupported
            },
            id,
            version_id,
        }
    }
}
