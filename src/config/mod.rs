//! TOML-driven configuration: the Rust replacement for the legacy
//! `src/Configs/{app,os}.php` PHP arrays.
//!
//! All sections implement [`Default`] and are tagged with `#[serde(default)]`,
//! so a partial user TOML file overlays on top of the built-in defaults
//! without needing manual coalescing logic.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub unit3d: Unit3dSection,
    #[serde(default)]
    pub app: AppSection,
    #[serde(default)]
    pub os: OsSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit3dSection {
    /// Minimum PHP version that the target box must run.
    #[serde(default = "default_min_php_version")]
    pub min_php_version: String,
    /// Git repository to clone.
    #[serde(default = "default_repository")]
    pub repository: String,
    /// Tag or branch to checkout.
    #[serde(default = "default_tag")]
    pub tag: String,
}

impl Default for Unit3dSection {
    fn default() -> Self {
        Self {
            min_php_version: default_min_php_version(),
            repository: default_repository(),
            tag: default_tag(),
        }
    }
}

fn default_min_php_version() -> String {
    "8.5".to_string()
}
fn default_repository() -> String {
    "https://github.com/HDInnovations/UNIT3D-Community-Edition.git".to_string()
}
fn default_tag() -> String {
    "v9.2.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSection {
    #[serde(default)]
    pub server_name: String,
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default = "default_ssl")]
    pub ssl: bool,
    #[serde(default = "default_branch")]
    pub branch: String,

    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub owner_email: String,
    #[serde(default)]
    pub password: String,

    #[serde(default = "default_db_driver")]
    pub db_driver: DbDriver,
    #[serde(default = "default_db_name")]
    pub db: String,
    #[serde(default = "default_db_user")]
    pub dbuser: String,
    #[serde(default)]
    pub dbpass: String,
    #[serde(default)]
    pub dbrootpass: String,

    #[serde(default = "default_mail_driver")]
    pub mail_driver: String,
    #[serde(default)]
    pub mail_host: String,
    #[serde(default = "default_mail_port")]
    pub mail_port: String,
    #[serde(default)]
    pub mail_username: String,
    #[serde(default)]
    pub mail_password: String,
    #[serde(default)]
    pub mail_from_name: String,

    #[serde(default = "default_echo_port")]
    pub echo_port: u16,
    #[serde(default)]
    pub tmdb_key: String,
    #[serde(default)]
    pub meilisearch_key: String,
}

impl Default for AppSection {
    fn default() -> Self {
        Self {
            server_name: String::new(),
            ip: String::new(),
            hostname: String::new(),
            ssl: default_ssl(),
            branch: default_branch(),
            owner: String::new(),
            owner_email: String::new(),
            password: String::new(),
            db_driver: default_db_driver(),
            db: default_db_name(),
            dbuser: default_db_user(),
            dbpass: String::new(),
            dbrootpass: String::new(),
            mail_driver: default_mail_driver(),
            mail_host: String::new(),
            mail_port: default_mail_port(),
            mail_username: String::new(),
            mail_password: String::new(),
            mail_from_name: String::new(),
            echo_port: default_echo_port(),
            tmdb_key: String::new(),
            meilisearch_key: String::new(),
        }
    }
}

fn default_ssl() -> bool {
    true
}
fn default_branch() -> String {
    "master".to_string()
}
fn default_db_driver() -> DbDriver {
    DbDriver::MariaDb
}
fn default_db_name() -> String {
    "unit3d".to_string()
}
fn default_db_user() -> String {
    "unit3d".to_string()
}
fn default_mail_driver() -> String {
    "smtp".to_string()
}
fn default_mail_port() -> String {
    "587".to_string()
}
fn default_echo_port() -> u16 {
    8443
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum DbDriver {
    Mysql,
    MariaDb,
    Postgres,
}

impl DbDriver {
    /// The Laravel `.env` `DB_CONNECTION` value.
    pub fn as_db_connection(&self) -> &'static str {
        match self {
            DbDriver::Mysql => "mysql",
            DbDriver::MariaDb => "mariadb",
            DbDriver::Postgres => "pgsql",
        }
    }

    /// The apt package name providing the server.
    #[allow(dead_code)]
    pub fn package(&self) -> &'static str {
        match self {
            DbDriver::Mysql => "mysql-server",
            DbDriver::MariaDb => "mariadb-server",
            DbDriver::Postgres => "postgresql",
        }
    }

    /// The admin CLI binary used to issue `CREATE DATABASE` / `CREATE USER`.
    #[allow(dead_code)]
    pub fn admin_binary(&self) -> &'static str {
        match self {
            DbDriver::Mysql => "mysql",
            DbDriver::MariaDb => "mariadb",
            DbDriver::Postgres => "psql",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OsSection {
    #[serde(default)]
    pub ubuntu: UbuntuOs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UbuntuOs {
    #[serde(default = "default_pkg_manager")]
    pub pkg_manager: String,
    #[serde(default = "default_web_user")]
    pub web_user: String,
    #[serde(default = "default_install_dir")]
    pub install_dir: PathBuf,
    #[serde(default = "default_nginx_sites")]
    pub nginx_sites_available_path: PathBuf,
    #[serde(default)]
    pub software: SoftwareSection,
}

impl Default for UbuntuOs {
    fn default() -> Self {
        Self {
            pkg_manager: default_pkg_manager(),
            web_user: default_web_user(),
            install_dir: default_install_dir(),
            nginx_sites_available_path: default_nginx_sites(),
            software: SoftwareSection::default(),
        }
    }
}

fn default_pkg_manager() -> String {
    "apt-get".to_string()
}
fn default_web_user() -> String {
    "www-data".to_string()
}
fn default_install_dir() -> PathBuf {
    PathBuf::from("/var/www/html")
}
fn default_nginx_sites() -> PathBuf {
    PathBuf::from("/etc/nginx/sites-available")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareSection {
    #[serde(default = "default_software")]
    pub packages: BTreeMap<String, String>,
    #[serde(default = "default_php_extensions")]
    pub php_extensions: Vec<String>,
}

impl Default for SoftwareSection {
    fn default() -> Self {
        Self {
            packages: default_software(),
            php_extensions: default_php_extensions(),
        }
    }
}

fn default_software() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    let items = [
        ("build-essential", "Basic C/C++ Development Environment"),
        ("nginx", "Web Server"),
        ("mariadb-server", "Database Server (MariaDB)"),
        ("mysql-server", "Database Server (MySQL)"),
        ("postgresql", "Database Server (PostgreSQL)"),
        ("supervisor", "A Process Control System"),
        ("nodejs", "JavaScript Run-time Environment (Includes npm)"),
        ("git", "Version Control"),
        ("tmux", "Screen Multiplexer"),
        ("vim", "Text Editor"),
        ("wget", "Transfer Data From A Server"),
        ("zip", "Compress Files"),
        ("unzip", "Decompress Files"),
        ("htop", "Monitor Server Resources"),
        ("redis-server", "Advanced Key-Value Store"),
        ("cron", "Process Scheduling Daemon"),
        ("acl", "Access Control Lists"),
        ("net-tools", "Network diagnostics"),
        ("gnupg", "GnuPG"),
        ("lsb-release", "LSB version info"),
        ("apt-transport-https", "HTTPS apt transport"),
        ("ca-certificates", "SSL certificates"),
        ("software-properties-common", "PPA management"),
        ("certbot", "Let's Encrypt SSL bot"),
        ("python3-certbot-nginx", "Certbot nginx plugin"),
    ];
    for (k, v) in items {
        m.insert(k.to_string(), v.to_string());
    }
    m
}

fn default_php_extensions() -> Vec<String> {
    [
        "php8.5-fpm",
        "php8.5-cli",
        "php8.5-mysql",
        "php8.5-pgsql",
        "php8.5-sqlite3",
        "php8.5-redis",
        "php8.5-memcached",
        "php8.5-curl",
        "php8.5-gd",
        "php8.5-imagick",
        "php8.5-mbstring",
        "php8.5-xml",
        "php8.5-zip",
        "php8.5-bcmath",
        "php8.5-intl",
        "php8.5-soap",
        "php8.5-opcache",
        "php8.5-readline",
        "php8.5-common",
        "php8.5-igbinary",
        "php8.5-msgpack",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed reading config file {0}: {1}")]
    Read(PathBuf, std::io::Error),
    #[error("failed parsing TOML config: {0}")]
    Parse(#[from] toml::de::Error),
}

impl Config {
    /// Load configuration from an optional TOML file. Missing sections and
    /// fields transparently fall back to the baked-in defaults via
    /// `#[serde(default)]`.
    pub fn load(maybe_path: Option<&Path>) -> Result<Self, ConfigError> {
        let mut text = String::new();
        if let Some(path) = maybe_path {
            text = std::fs::read_to_string(path)
                .map_err(|e| ConfigError::Read(path.to_path_buf(), e))?;
        }
        let cfg: Config = toml::from_str(&text)?;
        Ok(cfg)
    }

    /// Resolve the install dir, falling back to the OS section default.
    pub fn install_dir(&self) -> &Path {
        &self.os.ubuntu.install_dir
    }

    pub fn web_user(&self) -> &str {
        &self.os.ubuntu.web_user
    }
}
