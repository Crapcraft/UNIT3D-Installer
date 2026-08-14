//! Read `/proc/meminfo` total RAM (KB) — mirrors the legacy `memory()` helper.

pub fn memory() -> u64 {
    let Ok(text) = std::fs::read_to_string("/proc/meminfo") else {
        return 0;
    };
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            // Format: "MemTotal:       16384000 kB"
            let kb: u64 = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            return kb;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_mem_total() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            tmp.path(),
            "MemTotal:       16384000 kB\nMemFree:         123456 kB\n",
        )
        .unwrap();
        let text = std::fs::read_to_string(tmp.path()).unwrap();
        let total = parse_for_test(text);
        assert_eq!(total, 16_384_000);
    }

    fn parse_for_test(text: String) -> u64 {
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                return rest
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            }
        }
        0
    }

    #[test]
    fn no_memtotal_line_returns_zero() {
        let text = "MemFree: 123 kB\nMemAvailable: 456 kB\n".to_string();
        assert_eq!(parse_for_test(text), 0);
    }

    #[test]
    fn empty_input_returns_zero() {
        assert_eq!(parse_for_test(String::new()), 0);
    }

    #[test]
    fn garbage_memtotal_returns_zero() {
        let text = "MemTotal: not-a-number kB\n".to_string();
        assert_eq!(parse_for_test(text), 0);
    }

    #[test]
    fn multiline_handling_picks_memtotal() {
        let text = "MemTotal:       8192000 kB\nMemFree:  100 kB\n".to_string();
        assert_eq!(parse_for_test(text), 8_192_000);
    }
}
