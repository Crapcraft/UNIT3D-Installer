use askama::Template;

/// `/root/.my.cnf` — admin credentials file used by MySQL/MariaDB bootstrap
/// to invoke `mysql -e ...` without prompting for the root password.
#[derive(Debug, Clone, Template)]
#[template(path = "my.cnf", escape = "none")]
pub struct MyCnfTemplate<'a> {
    pub password: &'a str,
}
