use askama::Template;

/// Nginx `sites-available/<fqdn>` site file. Includes security headers, gzip,
/// `/socket.io` proxy to laravel-echo-server on the configured port,
/// static-asset caching, and sensitive-file denies.
#[derive(Debug, Clone, Template)]
#[template(path = "nginx.site", escape = "none")]
pub struct NginxTemplate<'a> {
    pub fqdn: &'a str,
    pub install_dir: &'a str,
    pub echo_port: u16,
    pub max_body: &'a str,
}
