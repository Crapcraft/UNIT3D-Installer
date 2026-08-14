use askama::Template;

/// `/etc/meilisearch.toml` — Meilisearch runtime config.
#[derive(Debug, Clone, Template)]
#[template(path = "meilisearch.toml", escape = "none")]
pub struct MeilisearchTomlTemplate<'a> {
    pub master_key: &'a str,
    pub db_path: &'a str,
    pub dump_dir: &'a str,
    pub snapshot_dir: &'a str,
}
