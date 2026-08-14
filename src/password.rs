//! Random-string generators. Replaces the legacy `str_random()` and
//! `bin2hex(random_bytes(16))` helpers.
//!
//! The character set is restricted to base64-stripped alphanumerics so the
//! outputs are safe to embed in shell strings, TOML values, `.env`,
//! MySQL `IDENTIFIED BY` clauses, and URLs without escaping.

use rand::Rng;

const ALNUM: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// Generate a random alphanumeric string of the given length.
pub fn str_random(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..ALNUM.len());
            ALNUM[idx] as char
        })
        .collect()
}

/// Generate a 32-char hex string (128 bits of entropy). Mirrors
/// `bin2hex(random_bytes(16))` from the PHP `ServerSetup`/`MeilisearchSetup`.
pub fn hex32() -> String {
    // `gen` is a reserved keyword in Edition 2024 — use `r#gen`.
    let bytes: [u8; 16] = rand::thread_rng().r#gen();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// If the given string is empty, generate a fresh `str_random(20)`. Mirrors
/// gap G1's auto-generation of MySQL/owner passwords when the user leaves
/// the prompt blank.
pub fn if_empty_generate(value: &mut String) {
    if value.is_empty() {
        *value = str_random(20);
    }
}

/// If the given string is empty, generate a fresh 32-char hex key (used by
/// the Meilisearch master key).
pub fn if_empty_generate_hex(value: &mut String) {
    if value.is_empty() {
        *value = hex32();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_random_len() {
        assert_eq!(str_random(20).len(), 20);
        assert_eq!(str_random(0).len(), 0);
    }

    #[test]
    fn str_random_alnum() {
        for c in str_random(64).chars() {
            assert!(c.is_ascii_alphanumeric());
        }
    }

    #[test]
    fn hex32_format() {
        let s = hex32();
        assert_eq!(s.len(), 32);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn if_empty_generate_replaces_empty() {
        let mut s = String::new();
        if_empty_generate(&mut s);
        assert!(!s.is_empty());
    }

    #[test]
    fn if_empty_generate_keeps_existing() {
        let mut s = String::from("hunter2");
        if_empty_generate(&mut s);
        assert_eq!(s, "hunter2");
    }

    #[test]
    fn if_empty_generate_hex_keeps_existing() {
        let mut s = String::from("custom-key");
        if_empty_generate_hex(&mut s);
        assert_eq!(s, "custom-key");
    }

    #[test]
    fn if_empty_generate_hex_replaces_empty() {
        let mut s = String::new();
        if_empty_generate_hex(&mut s);
        assert_eq!(s.len(), 32);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn generated_passwords_differ() {
        // Extremely unlikely to collide; guards against degenerate RNG.
        let a = str_random(20);
        let b = str_random(20);
        assert_ne!(a, b);
    }

    #[test]
    fn passwords_have_no_ambiguous_characters() {
        // Shell/shell-embedded-safe: only alphanumerics, no quotes/spaces.
        for c in str_random(100).chars() {
            assert!(c.is_ascii_alphanumeric(), "unsafe char {c:?}");
        }
    }

    #[test]
    fn hex32_lowercase() {
        let s = hex32();
        assert_eq!(s, s.to_ascii_lowercase());
    }
}
