//! UNIT3D-Community-Edition installer (Rust port) — binary crate.
//!
//! Thin wrapper around the library crate's [`unit3d_installer::run`].
//! CLI parsing (and `--help`/`--version` handling) happens inside `run()`.

fn main() {
    if let Err(e) = unit3d_installer::run() {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}
