//! Secret redaction for log output.
//!
//! Every shell command is logged via `tracing::info!` in [`crate::process`].
//! Commands embed credentials directly (e.g. `mysqladmin -u root password
//! hunter2`, `CREATE USER ... IDENTIFIED BY 'hunter2'`), so we scrub known
//! secret patterns from anything that is persisted or printed for
//! diagnostics.

const REPLACEMENT: &str = "<redacted>";

/// Replace the sensitive values in a command line with a redaction marker.
///
/// Handles the password-bearing patterns this installer emits:
/// - `password <value>` / `password='<value>'`
/// - `IDENTIFIED BY '<value>'`
/// - `PASSWORD '<value>'`
/// - `KEY=value` assignments for known secret keys (`APP_KEY`,
///   `DB_PASSWORD`, `MAIL_PASSWORD`, `MEILISEARCH_MASTER_KEY`, ...)
///
/// Everything else is passed through verbatim so operators can still read
/// the full command line.
pub fn redact(cmd: &str) -> String {
    let mut out = cmd.to_string();
    out = scrub_phrase(&out, "password");
    out = scrub_phrase(&out, "IDENTIFIED BY");
    out = scrub_phrase(&out, "PASSWORD");
    for key in [
        "APP_KEY",
        "DB_PASSWORD",
        "DB_ROOT_PASSWORD",
        "DBPASSWORD",
        "MAIL_PASSWORD",
        "MEILISEARCH_MASTER_KEY",
        "MEILISEARCH_KEY",
        "PASSWORD",
    ] {
        out = scrub_assignment(&out, key);
    }
    out
}

/// True when `out` has a word-boundary character before `start` — i.e. the
/// phrase is a standalone word and not the tail of a longer identifier like
/// `passwordless` or `my_password_field`.
fn has_word_boundary_before(out: &str, start: usize) -> bool {
    if start == 0 {
        return true;
    }
    let Some(prev) = out[..start].chars().next_back() else {
        return true;
    };
    !(prev.is_ascii_alphanumeric() || prev == '_')
}

/// Scrub `<phrase> <value>` or `<phrase>='<value>'` occurrences where the
/// value is a standalone word after the phrase. Stops at whitespace, quotes,
/// `;`, backticks, or `)`. Uses an advancing cursor so each phrase match is
/// processed exactly once (no infinite loops).
fn scrub_phrase(s: &str, phrase: &str) -> String {
    let mut out = s.to_string();
    let mut cursor = 0;
    while let Some(rel) = out.get(cursor..).and_then(|rest| rest.find(phrase)) {
        let start = cursor + rel;
        if !has_word_boundary_before(&out, start) {
            cursor = start + phrase.len();
            continue;
        }
        let after = start + phrase.len();
        let Some(rest) = out.get(after..) else {
            break;
        };
        // A trailing alphanumeric/_ means the phrase is a word prefix (e.g.
        // "passwordless"); don't scrub it.
        if rest
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            cursor = after;
            continue;
        }
        // Skip an optional '='.
        let after_eq = if rest.starts_with('=') {
            after + 1
        } else {
            after
        };
        // Skip leading whitespace to reach the value.
        let Some(after_space) = out
            .get(after_eq..)
            .map(|r| r.len() - r.trim_start().len())
            .map(|n| after_eq + n)
        else {
            break;
        };
        let Some(slice) = out.get(after_space..) else {
            break;
        };
        let bytes = slice.as_bytes();
        // The value must be a real token (not a bare quote/end).
        if bytes.is_empty() {
            cursor = after;
            continue;
        }
        // Skip an opening quote.
        let mut idx = 0;
        let mut quote = None;
        if matches!(bytes.first(), Some(b'\'') | Some(b'"')) {
            quote = bytes.first().copied();
            idx = 1;
        }
        let mut end = idx;
        while idx < bytes.len() {
            match bytes[idx] {
                b if quote == Some(b) => {
                    end = idx + 1;
                    break;
                }
                b' ' | b'\t' | b'\n' | b';' | b'`' | b')' if quote.is_none() => break,
                _ => end = idx + 1,
            }
            idx += 1;
        }
        if end == 0 {
            // No scrubbable value; advance past the phrase to keep moving.
            cursor = after;
            continue;
        }
        let before = &out[..after_space];
        let after_rep = &out[after_space + end..];
        out = format!("{before}{REPLACEMENT}{after_rep}");
        cursor = after_space + REPLACEMENT.len();
    }
    out
}

/// Scrub `KEY=<value>` assignments where the value may be quoted and stops
/// at whitespace or a quote.
fn scrub_assignment(s: &str, key: &str) -> String {
    let mut out = s.to_string();
    let mut cursor = 0;
    while let Some(rel) = out.get(cursor..).and_then(|rest| rest.find(key)) {
        let start = cursor + rel;
        if !has_word_boundary_before(&out, start) {
            cursor = start + key.len();
            continue;
        }
        let after = start + key.len();
        let Some(rest) = out.get(after..) else {
            break;
        };
        if !rest.starts_with('=') {
            cursor = after;
            continue;
        }
        let val_start = after + 1;
        let Some(bytes) = out.get(val_start..).map(|s| s.as_bytes()) else {
            break;
        };
        if bytes.is_empty() {
            cursor = after;
            continue;
        }
        let mut idx = 0;
        let mut quote = None;
        if matches!(bytes.first(), Some(b'\'') | Some(b'"')) {
            quote = bytes.first().copied();
            idx = 1;
        }
        let mut end = idx;
        while idx < bytes.len() {
            match bytes[idx] {
                b if quote == Some(b) => {
                    end = idx + 1;
                    break;
                }
                b' ' | b'\t' | b'\n' | b';' | b'`' | b')' if quote.is_none() => break,
                _ => end = idx + 1,
            }
            idx += 1;
        }
        if end == 0 {
            cursor = after;
            continue;
        }
        let before = &out[..val_start];
        let after_rep = &out[val_start + end..];
        out = format!("{before}{REPLACEMENT}{after_rep}");
        cursor = val_start + REPLACEMENT.len();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_mysqladmin_password() {
        let cmd = "mysqladmin -u root password hunter2";
        let out = redact(cmd);
        assert!(!out.contains("hunter2"), "got: {out}");
        assert!(out.contains("<redacted>"));
    }

    #[test]
    fn redacts_identified_by_single_quotes() {
        let cmd = "mysql -e \"CREATE USER 'unit3d'@'localhost' IDENTIFIED BY 'secretpass'\"";
        let out = redact(cmd);
        assert!(!out.contains("secretpass"), "got: {out}");
        assert!(out.contains("<redacted>"));
    }

    #[test]
    fn redacts_postgres_password() {
        let cmd = "echo 'CREATE ROLE unit3d LOGIN PASSWORD secretpass' | sudo -u postgres psql";
        let out = redact(cmd);
        assert!(!out.contains("secretpass"), "got: {out}");
    }

    #[test]
    fn redacts_env_assignments() {
        let cmd = "DB_PASSWORD=hunter2 APP_KEY=base64:abc123";
        let out = redact(cmd);
        assert!(!out.contains("hunter2"), "got: {out}");
        assert!(!out.contains("abc123"), "got: {out}");
        assert!(out.contains("<redacted>"));
    }

    #[test]
    fn leaves_innocuous_commands_alone() {
        let cmd = "apt-get install -y nginx redis-server";
        assert_eq!(redact(cmd), cmd);
    }

    #[test]
    fn redacts_quoted_password_token() {
        let cmd = "mysqladmin -u root password 'rootpw'";
        let out = redact(cmd);
        assert!(!out.contains("rootpw"), "got: {out}");
        assert!(out.contains("<redacted>"));
    }

    #[test]
    fn no_panic_on_empty() {
        assert_eq!(redact(""), "");
    }

    #[test]
    fn redacts_equals_spelling() {
        let cmd = "mysqladmin -u root password='hunter2'";
        let out = redact(cmd);
        assert!(!out.contains("hunter2"), "got: {out}");
    }

    #[test]
    fn leaves_host_and_database_names() {
        let cmd = "mysql -e \"GRANT ALL PRIVILEGES ON unit3d . * TO 'unit3d'@'localhost'\"";
        let out = redact(cmd);
        assert!(out.contains("unit3d"), "db/user names must survive: {out}");
    }

    #[test]
    fn scrub_assignment_handles_bare_and_quoted() {
        assert_eq!(scrub_assignment("A=B", "A"), "A=<redacted>");
        assert_eq!(scrub_assignment("A='x y'", "A"), "A=<redacted>");
        assert_eq!(scrub_assignment("A=\"z\"", "A"), "A=<redacted>");
    }

    #[test]
    fn scrub_phrase_stops_at_semicolon() {
        assert_eq!(
            scrub_phrase("password p; echo hi", "password"),
            "password <redacted>; echo hi"
        );
    }

    #[test]
    fn multiple_secrets_all_redacted() {
        let cmd =
            "mysqladmin -u root password hunter2; mysql -e \"IDENTIFIED BY 'x'\" ; DB_PASSWORD=zzz";
        let out = redact(cmd);
        assert!(!out.contains("hunter2"), "got: {out}");
        assert!(!out.contains("'x'"), "got: {out}");
        assert!(!out.contains("zzz"), "got: {out}");
        assert_eq!(out.matches("<redacted>").count(), 3, "got: {out}");
    }

    #[test]
    fn repeated_phrase_handled() {
        let out = redact("password a password b password c");
        assert!(!out.contains("a password b"), "got: {out}");
        assert_eq!(out.matches("<redacted>").count(), 3, "got: {out}");
    }

    #[test]
    fn double_dash_password_style() {
        // `--password=secret` should be redacted.
        let out = redact("mysql --password=hunter2");
        assert!(!out.contains("hunter2"), "got: {out}");
        assert!(out.contains("<redacted>"), "got: {out}");
    }

    #[test]
    fn dash_p_short_style_is_not_mangled() {
        // We don't try to parse `-pSECRET` (ambiguous with flags); just make
        // sure we don't corrupt other tokens.
        let out = redact("echo -p hello");
        assert!(out.contains("-p hello"), "got: {out}");
    }

    #[test]
    fn no_false_positive_on_passwordless() {
        let out = redact("echo passwordless auth");
        assert_eq!(out, "echo passwordless auth", "got: {out}");
    }

    #[test]
    fn no_false_positive_on_underscore_field() {
        let out = redact("check my_password_field here");
        assert_eq!(out, "check my_password_field here", "got: {out}");
    }

    #[test]
    fn value_with_spaces_in_assignment() {
        // Quoted values keep their spaces inside the redaction span.
        let out = redact("MAIL_PASSWORD='hush hush' rest");
        assert!(!out.contains("hush"), "got: {out}");
        assert!(out.contains("rest"), "got: {out}");
    }

    #[test]
    fn value_with_equals_inside_quotes() {
        let out = redact("DB_PASSWORD='a=b' tail");
        assert!(!out.contains("a=b"), "got: {out}");
        assert!(out.contains("tail"), "got: {out}");
    }

    #[test]
    fn phrase_at_end_of_string() {
        let out = redact("mysqladmin -u root password");
        // No value follows; the phrase stays intact (nothing to leak).
        assert_eq!(out, "mysqladmin -u root password");
    }

    #[test]
    fn phrase_with_empty_equals_value() {
        let out = redact("password=");
        assert_eq!(out, "password=", "got: {out}");
        // A value that follows an '=' is still a credential and is scrubbed.
        let out2 = redact("password= rest");
        assert!(!out2.contains("rest"), "got: {out2}");
    }

    #[test]
    fn utf8_values_survive() {
        // Multi-byte values are scrubbed wholesale (not partially).
        let out = redact("password 'pa🚀ss'");
        assert!(!out.contains("🚀"), "got: {out}");
        assert!(!out.contains("'pa"), "got: {out}");
        assert!(out.contains("<redacted>"), "got: {out}");
    }

    #[test]
    fn meilisearch_key_redacted() {
        let out = redact("MEILISEARCH_MASTER_KEY=0870969f41be10b8e252da7da13330b0");
        assert!(!out.contains("0870969f"), "got: {out}");
        assert!(out.contains("<redacted>"), "got: {out}");
    }

    #[test]
    fn app_key_redacted() {
        let out = redact("APP_KEY=base64:Q3VtUmFuZG9tTWVyZQ==");
        assert!(!out.contains("Q3Vt"), "got: {out}");
        assert!(out.contains("<redacted>"), "got: {out}");
    }

    #[test]
    fn consecutive_key_value_pairs_both_redacted() {
        let out = redact("DB_PASSWORD=first MAIL_PASSWORD=second");
        assert!(!out.contains("first"), "got: {out}");
        assert!(!out.contains("second"), "got: {out}");
        assert_eq!(out.matches("<redacted>").count(), 2, "got: {out}");
    }

    #[test]
    fn password_value_right_after_phrase_no_space() {
        let out = redact("mysqladmin -u root password'jump'");
        assert!(!out.contains("jump"), "got: {out}");
        assert!(out.contains("<redacted>"), "got: {out}");
    }
}
