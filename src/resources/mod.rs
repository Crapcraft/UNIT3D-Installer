//! Askama-compiled templates that replace the legacy `src/Resources/*` PHP
//! stubs. Each template lives under `templates/` and is rendered at compile
//! time (no runtime template engine dependency).

pub mod credentials;
pub mod echo_server;
pub mod env;
pub mod intro;
pub mod meilisearch_toml;
pub mod meilisearch_unit;
pub mod my_cnf;
pub mod nginx;
pub mod phpfpm;
pub mod supervisor;
