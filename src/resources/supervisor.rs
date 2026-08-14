use askama::Template;

/// `/etc/supervisor/conf.d/unit3d.conf` — queue workers + Laravel Echo Server.
#[derive(Debug, Clone, Template)]
#[template(path = "supervisor.conf", escape = "none")]
pub struct SupervisorTemplate<'a> {
    pub install_dir: &'a str,
    pub web_user: &'a str,
}
