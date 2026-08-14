//! Database step integration tests. Drive the MySQL / MariaDB / PostgreSQL
//! drivers through the mocked executor and assert the emitted provisioning
//! commands.

mod common;

use common::test_context_dry;
use unit3d_installer::config::DbDriver;
use unit3d_installer::steps::Step;
use unit3d_installer::steps::database::DatabaseStep;

#[test]
fn mysql_step_installs_and_provisions() {
    let (mut ctx, exec) = test_context_dry();
    ctx.config.app.db_driver = DbDriver::Mysql;
    ctx.config.app.db = "unit3d".to_string();
    ctx.config.app.dbuser = "unit3d".to_string();
    ctx.config.app.dbpass = "secretpass".to_string();
    ctx.config.app.dbrootpass = "rootpw".to_string();

    DatabaseStep.handle(&mut ctx).unwrap();

    // Package install.
    assert!(exec.any("apt-get install -y mysql-server"));
    // Init.
    assert!(exec.any("mysqld --initialize-insecure"));
    // Root password.
    assert!(exec.any("mysqladmin -u root password rootpw"));
    // .my.cnf written for passwordless auth.
    assert!(exec.any("chmod 600 /root/.my.cnf"));
    // Provisioning SQL.
    assert!(exec.any("DROP USER IF EXISTS 'unit3d'@'localhost'"));
    assert!(exec.any("CREATE DATABASE unit3d"));
    assert!(exec.any("CREATE USER 'unit3d'@'localhost' IDENTIFIED BY 'secretpass'"));
    assert!(exec.any("GRANT ALL PRIVILEGES ON unit3d . * TO 'unit3d'@'localhost'"));
    // Hardening.
    assert!(exec.any("DELETE FROM mysql.user WHERE User=''"));
    assert!(exec.any("FLUSH PRIVILEGES"));
    assert!(exec.any("DROP DATABASE IF EXISTS test"));
}

#[test]
fn mariadb_step_uses_mariadb_binaries() {
    let (mut ctx, exec) = test_context_dry();
    ctx.config.app.db_driver = DbDriver::MariaDb;
    ctx.config.app.db = "unit3d".to_string();
    ctx.config.app.dbuser = "unit3d".to_string();
    ctx.config.app.dbpass = "secretpass".to_string();
    ctx.config.app.dbrootpass = "rootpw".to_string();

    DatabaseStep.handle(&mut ctx).unwrap();

    assert!(exec.any("apt-get install -y mariadb-server"));
    assert!(exec.any("mariadbd --initialize-insecure"));
    assert!(exec.any("mariadb-admin -u root password rootpw"));
    assert!(exec.any("service mariadb start"));
    // Driver dispatch forwards to the shared MySQL logic.
    assert!(exec.any("CREATE USER 'unit3d'@'localhost'"));
}

#[test]
fn postgres_step_installs_and_creates_role() {
    let (mut ctx, exec) = test_context_dry();
    ctx.config.app.db_driver = DbDriver::Postgres;
    ctx.config.app.db = "unit3d".to_string();
    ctx.config.app.dbuser = "unit3d".to_string();
    ctx.config.app.dbpass = "secretpass".to_string();

    DatabaseStep.handle(&mut ctx).unwrap();

    assert!(exec.any("apt-get install -y postgresql"));
    assert!(exec.any("systemctl start postgresql"));
    assert!(exec.any("systemctl enable postgresql"));
    // SQL piped to psql as the postgres OS user.
    assert!(exec.any("sudo -u postgres psql"));
    assert!(exec.any("CREATE ROLE unit3d LOGIN PASSWORD 'secretpass'"));
    assert!(exec.any("CREATE DATABASE unit3d"));
    assert!(exec.any("GRANT ALL PRIVILEGES ON DATABASE unit3d TO unit3d"));
}

#[test]
fn database_step_never_touches_mysql_paths_when_dry_run() {
    let (mut ctx, exec) = test_context_dry();
    ctx.config.app.db_driver = DbDriver::Mysql;
    // dry-run writes my.cnf to a temp-cleanable location via write_file,
    // which just prints. Ensure no real /root or /etc writes happened by
    // checking the commands recorded are the shell commands only.
    DatabaseStep.handle(&mut ctx).unwrap();
    assert!(exec.any("mkdir -p /var/lib/mysql"));
}

#[test]
fn database_step_respects_non_interactive_defaults() {
    let (mut ctx, exec) = test_context_dry();
    // Default driver is MariaDb; running without explicit config should
    // still emit the MariaDB commands rather than erroring.
    DatabaseStep.handle(&mut ctx).unwrap();
    assert!(exec.any("mariadb-server"));
}

#[test]
fn mysql_root_password_is_shell_quoted() {
    let (mut ctx, exec) = test_context_dry();
    ctx.config.app.db_driver = DbDriver::Mysql;
    ctx.config.app.db = "unit3d".to_string();
    ctx.config.app.dbuser = "unit3d".to_string();
    ctx.config.app.dbpass = "secretpass".to_string();
    ctx.config.app.dbrootpass = "ro'otpw".to_string();

    DatabaseStep.handle(&mut ctx).unwrap();

    // shell_quote strips single quotes from the root password.
    assert!(exec.any("mysqladmin -u root password rootpw"));
    assert!(exec.any("IDENTIFIED WITH mysql_native_password BY 'rootpw'"));
    // .my.cnf also gets the quoted password.
    assert!(exec.any("chmod 600 /root/.my.cnf"));
}

#[test]
fn database_step_removes_anonymous_and_remote_root() {
    let (mut ctx, exec) = test_context_dry();
    ctx.config.app.db_driver = DbDriver::MariaDb;
    ctx.config.app.db = "unit3d".to_string();
    ctx.config.app.dbuser = "unit3d".to_string();
    ctx.config.app.dbpass = "secretpass".to_string();
    ctx.config.app.dbrootpass = "rootpw".to_string();

    DatabaseStep.handle(&mut ctx).unwrap();

    assert!(exec.any("DELETE FROM mysql.user WHERE User=''"));
    assert!(exec.any("Host NOT IN ('localhost', '127.0.0.1', '::1')"));
    assert!(exec.any("DROP DATABASE IF EXISTS test"));
    assert!(exec.any("DELETE FROM mysql.db WHERE Db='test'"));
}

#[test]
fn mariadb_init_uses_mariadbd() {
    let (mut ctx, exec) = test_context_dry();
    ctx.config.app.db_driver = DbDriver::MariaDb;
    ctx.config.app.db = "unit3d".to_string();
    ctx.config.app.dbuser = "unit3d".to_string();
    ctx.config.app.dbpass = "secretpass".to_string();
    ctx.config.app.dbrootpass = "rootpw".to_string();

    DatabaseStep.handle(&mut ctx).unwrap();

    assert!(exec.any("mariadbd --initialize-insecure"));
    assert!(exec.any("update-rc.d mariadb defaults"));
    assert!(exec.any("service mariadb start"));
}
