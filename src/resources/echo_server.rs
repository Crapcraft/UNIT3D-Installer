use askama::Template;

/// `laravel-echo-server.json` written at the install root.
/// Mirrors `src/Resources/laravel-echo-server.stub`.
#[derive(Debug, Clone, Template)]
#[template(path = "laravel-echo-server.json", escape = "none")]
pub struct EchoServerTemplate<'a> {
    pub protocol: &'a str,
    pub fqdn: &'a str,
    pub port: u16,
    pub ssl_cert: &'a str,
    pub ssl_key: &'a str,
    pub ssl_chain: &'a str,
}
