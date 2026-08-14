use askama::Template;

/// PHP-FPM per-FQDN pool configuration (replaces
/// `src/Resources/ubuntu/php-fpm/php-fpm.conf`).
#[derive(Debug, Clone, Template)]
#[template(path = "php-fpm.conf", escape = "none")]
pub struct PhpFpmTemplate<'a> {
    pub fqdn: &'a str,
    pub web_user: &'a str,
}
