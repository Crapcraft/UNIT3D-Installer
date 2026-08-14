use askama::Template;

/// `/root/unit3d-credentials.txt` — saved ledger written after install.
#[derive(Debug, Clone, Template)]
#[template(path = "credentials.txt", escape = "none")]
pub struct CredentialsTemplate<'a> {
    pub generated: &'a str,
    pub fqdn: &'a str,
    pub owner: &'a str,
    pub owner_email: &'a str,
    pub owner_password: &'a str,
    pub db_name: &'a str,
    pub db_user: &'a str,
    pub db_pass: &'a str,
    pub db_root_pass: &'a str,
    pub meilisearch_key: &'a str,
    pub install_dir: &'a str,
    pub php_version: &'a str,
    pub web_user: &'a str,
}
