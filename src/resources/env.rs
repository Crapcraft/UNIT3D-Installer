use askama::Template;

/// `.env` file written into the cloned UNIT3D checkout. Mirrors the legacy
/// `src/Resources/.env.stub` with all the `{{UPPER}}` placeholders resolved
/// by the step.
#[derive(Debug, Clone, Template)]
#[template(path = "env.stub", escape = "none")]
pub struct EnvTemplate<'a> {
    pub protocol: &'a str,
    pub fqdn: &'a str,
    pub db_driver: &'a str,
    pub db: &'a str,
    pub dbuser: &'a str,
    pub dbpass: &'a str,
    pub socket: &'a str,
    pub owner: &'a str,
    pub owner_email: &'a str,
    pub owner_password: &'a str,
    pub tmdb_key: &'a str,
    pub mail_driver: &'a str,
    pub mail_host: &'a str,
    pub mail_port: &'a str,
    pub mail_username: &'a str,
    pub mail_password: &'a str,
    pub mail_from_name: &'a str,
    pub meilisearch_key: &'a str,
    pub redis_host: &'a str,
    pub redis_port: &'a str,
}
