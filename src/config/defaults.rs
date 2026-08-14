//! Built-in configuration defaults (kept separate from the public API so
//! the module `mod.rs` stays readable).
//!
//! These mirror the legacy `src/Configs/{app,os}.php` PHP arrays and the
//! v1.2 standalone script's canonical configuration values.

// No runtime symbols here: all defaults are baked into the `Default`
// impls in `mod.rs` via `#[serde(default = "...")]`. This file is kept as
// a placeholder for future defaulted constants.