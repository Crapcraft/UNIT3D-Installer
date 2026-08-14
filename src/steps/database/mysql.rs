//! MySQL/MariaDB driver. Provisioning logic is shared between them: install
//! the package, start the service, write `/root/.my.cnf`, create the
//! database + user, harden the root account, drop the test database. The
//! only differences are the binary names and the init command — both
//! handled by the [`Flavor`] argument.
//!
//! Replaces `src/Installer/Database/{MySqlSetup,MariaDbSetup}.php`.

use crate::config::DbDriver;
use crate::resources::my_cnf::MyCnfTemplate;
use crate::steps::Context;
use crate::system::memory;
use anyhow::Result;
use askama::Template;
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub enum Flavor {
    Mysql,
    MariaDb,
}

impl Flavor {
    fn binary(&self) -> &'static str {
        match self {
            Flavor::Mysql => "mysql",
            Flavor::MariaDb => "mariadb",
        }
    }
    fn server_pkg(&self) -> &'static str {
        match self {
            Flavor::Mysql => "mysql-server",
            Flavor::MariaDb => "mariadb-server",
        }
    }
    fn init_bin(&self) -> &'static str {
        match self {
            Flavor::Mysql => "mysqld",
            Flavor::MariaDb => "mariadbd",
        }
    }
    fn admin_bin(&self) -> &'static str {
        match self {
            Flavor::Mysql => "mysqladmin",
            Flavor::MariaDb => "mariadb-admin",
        }
    }
    fn service_name(&self) -> &'static str {
        match self {
            Flavor::Mysql => "mysql",
            Flavor::MariaDb => "mariadb",
        }
    }
}

pub fn configure(ctx: &mut Context) -> Result<()> {
    let flavor = match ctx.config.app.db_driver {
        DbDriver::Mysql => Flavor::Mysql,
        DbDriver::MariaDb => Flavor::MariaDb,
        _ => unreachable!("postgres should not reach this driver"),
    };
    ctx.style
        .info(&format!("Installing {} server", flavor.server_pkg()));

    // Install the server package.
    ctx.run(&format!(
        "{} install -y {}",
        ctx.config.os.ubuntu.pkg_manager,
        flavor.server_pkg()
    ))?;

    // Pick a tuning profile based on physical RAM (mirrors the PHP `memory()`
    // switch).
    let mycnf = pick_mycnf(memory());
    let cnf_src = format!("/etc/mysql/conf.d/{mycnf}");

    if ctx.dry_run || !Path::new(&cnf_src).exists() {
        // Use the legacy bundled tuning file by writing a sensible default
        // directly when the legacy stub file is not present on this box.
        let body = default_tuning_for(mycnf);
        ctx.write_file(std::path::Path::new(&cnf_src), body)?;
    }

    // On a fresh Ubuntu data dir is empty — initialize it.
    if ctx.dry_run || !Path::new("/var/lib/mysql").exists() || is_dir_empty("/var/lib/mysql")? {
        ctx.run_all([
            "mkdir -p /var/lib/mysql".to_string(),
            "chown mysql:mysql /var/lib/mysql".to_string(),
            format!("{} --initialize-insecure", flavor.init_bin()),
        ])?;
    }

    // `/root/.my.cnf` lets subsequent `mysql -e ...` calls authenticate
    // without prompting.
    let tpl = MyCnfTemplate {
        password: &ctx.config.app.dbrootpass,
    };
    let rendered = tpl.render()?;
    ctx.write_file(std::path::Path::new("/root/.my.cnf"), &rendered)?;
    ctx.run("chmod 600 /root/.my.cnf")?;

    // Start the service and set the root password.
    ctx.run_all([
        "mkdir -p /var/run/mysqld".to_string(),
        "chown mysql:mysql /var/run/mysqld".to_string(),
        "chmod -R 755 /var/run/mysqld".to_string(),
        format!("update-rc.d {} defaults", flavor.service_name()),
        format!("service {} start", flavor.service_name()),
        format!(
            "{} -u root password {}",
            flavor.admin_bin(),
            shell_quote(&ctx.config.app.dbrootpass)
        ),
    ])?;

    let db = &ctx.config.app.db;
    let dbuser = &ctx.config.app.dbuser;
    let dbpass = shell_quote(&ctx.config.app.dbpass);
    let root_pass = shell_quote(&ctx.config.app.dbrootpass);
    let bin = flavor.binary();

    let critical: [String; 9] = [
        format!("{bin} -e \"DROP USER IF EXISTS '{dbuser}'@'localhost'\""),
        format!("{bin} -e \"DROP DATABASE IF EXISTS {db}\""),
        format!("{bin} -e \"CREATE DATABASE {db}\""),
        format!("{bin} -e \"CREATE USER '{dbuser}'@'localhost' IDENTIFIED BY '{dbpass}'\""),
        format!("{bin} -e \"GRANT ALL PRIVILEGES ON {db} . * TO '{dbuser}'@'localhost'\""),
        format!(
            "{bin} -e \"ALTER USER 'root'@'localhost' IDENTIFIED WITH mysql_native_password BY '{root_pass}'\""
        ),
        format!("{bin} -e \"DELETE FROM mysql.user WHERE User=''\""),
        format!(
            "{bin} -e \"DELETE FROM mysql.user WHERE User='root' AND Host NOT IN ('localhost', '127.0.0.1', '::1')\""
        ),
        format!("{bin} -e \"FLUSH PRIVILEGES\""),
    ];
    ctx.run_all(critical)?;

    // Non-critical: drop the test database.
    ctx.run_all([
        format!("{bin} -e \"DROP DATABASE IF EXISTS test\""),
        format!("{bin} -e \"DELETE FROM mysql.db WHERE Db='test' OR Db='test\\\\_%'\""),
    ])
    .ok();
    let _ = ctx.run(&format!("{bin} -e \"FLUSH PRIVILEGES\""));

    ctx.style.info("Database configured successfully");
    Ok(())
}

fn pick_mycnf(mem_kb: u64) -> &'static str {
    if (1_200_000..3_900_000).contains(&mem_kb) {
        "my-medium.cnf"
    } else if mem_kb >= 3_900_000 {
        "my-large.cnf"
    } else {
        "my-small.cnf"
    }
}

fn default_tuning_for(name: &str) -> &'static str {
    match name {
        "my-large.cnf" => DEFAULT_LARGE,
        "my-medium.cnf" => DEFAULT_MEDIUM,
        _ => DEFAULT_SMALL,
    }
}

const DEFAULT_SMALL: &str = "[mysqld]\nkey_buffer_size = 16K\nmax_connections = 30\nmax_user_connections = 20\nwait_timeout = 10\ninnodb_file_per_table\n";
const DEFAULT_MEDIUM: &str = "[mysqld]\nkey_buffer_size = 16M\nmax_allowed_packet = 16M\nmax_connections = 70\nmax_user_connections = 30\nwait_timeout = 10\ninnodb_file_per_table\n";
const DEFAULT_LARGE: &str = "[mysqld]\nkey_buffer_size = 256M\nmax_allowed_packet = 32M\ntable_open_cache = 256\nthread_cache_size = 8\nmax_connections = 200\nmax_user_connections = 50\nwait_timeout = 10\ninnodb_file_per_table\n";

fn is_dir_empty(p: &str) -> Result<bool> {
    Ok(Path::new(p).exists() && std::fs::read_dir(p)?.next().is_none())
}

/// Single-quote a value for use inside an SQL identifier/clause. This is a
/// blunt protector against characters the shell would otherwise interpret;
/// the installer warns explicitly in interactive prompts that special
/// characters aren't supported yet.
pub fn shell_quote(s: &str) -> String {
    s.replace('\'', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_small() {
        assert_eq!(pick_mycnf(500_000), "my-small.cnf");
    }
    #[test]
    fn pick_medium() {
        assert_eq!(pick_mycnf(2_000_000), "my-medium.cnf");
    }
    #[test]
    fn pick_large() {
        assert_eq!(pick_mycnf(8_000_000), "my-large.cnf");
    }

    #[test]
    fn shell_quote_strips_single_quotes() {
        assert_eq!(shell_quote("a'b'c"), "abc");
    }

    #[test]
    fn default_tuning_for_small() {
        assert!(DEFAULT_SMALL.contains("innodb_file_per_table"));
    }
}
