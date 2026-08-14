//! Top-level credentials façade referenced by `main.rs`. The actual work
//! is performed by `steps::credentials::CredentialsStep`; this module
//! currently has no API of its own but is kept as a stable place for
//! future cross-cutting helpers (e.g. a `read_saved()` API).

#[allow(unused_imports)]
pub use crate::steps::credentials::CredentialsStep;
