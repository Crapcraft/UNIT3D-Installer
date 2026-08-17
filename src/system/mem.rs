//! Read `/proc/meminfo` total RAM (KB) — mirrors the legacy `memory()` helper.

pub fn memory() -> u64 {
    let Ok(text) = std::fs::read_to_string("/proc/meminfo") else {
        return 0;
    };
    parse_meminfo(&text)
}

/// Extract the `MemTotal` value (in KiB) from `/proc/meminfo` text.
/// Returns 0 if the line is missing or unparseable.
fn parse_meminfo(text: &str) -> u64 {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            // Format: "MemTotal:       16384000 kB"
            return rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mem_total() {
        let text = "MemTotal:       16384000 kB\nMemFree:         123456 kB\n";
        assert_eq!(parse_meminfo(text), 16_384_000);
    }

    #[test]
    fn no_memtotal_line_returns_zero() {
        let text = "MemFree: 123 kB\nMemAvailable: 456 kB\n";
        assert_eq!(parse_meminfo(text), 0);
    }

    #[test]
    fn empty_input_returns_zero() {
        assert_eq!(parse_meminfo(""), 0);
    }

    #[test]
    fn garbage_memtotal_returns_zero() {
        let text = "MemTotal: not-a-number kB\n";
        assert_eq!(parse_meminfo(text), 0);
    }

    #[test]
    fn multiline_handling_picks_memtotal() {
        let text = "MemTotal:       8192000 kB\nMemFree:  100 kB\n";
        assert_eq!(parse_meminfo(text), 8_192_000);
    }
}
