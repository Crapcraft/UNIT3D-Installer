use askama::Template;

/// `/etc/systemd/system/meilisearch.service` — systemd unit for Meilisearch.
#[derive(Debug, Clone, Template)]
#[template(path = "meilisearch.service", escape = "none")]
pub struct MeilisearchUnitTemplate<'a> {
    pub web_user: &'a str,
}
