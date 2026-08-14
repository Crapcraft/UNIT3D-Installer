//! PostgreSQL driver — new in the Rust port (selected via the
//! "MySQL + MariaDB + PostgreSQL" decision in the plan).
//!
//! Uses `sudo -u postgres psql` for the privileged admin commands, then
//! creates the application role + database with `OWNER = <unit3d user>`.

use crate::steps::Context;
use anyhow::Result;

pub fn configure(ctx: &mut Context) -> Result<()> {
    ctx.style.info("Installing PostgreSQL server");
    ctx.run(&format!(
        "{} install -y postgresql",
        ctx.config.os.ubuntu.pkg_manager
    ))?;

    ctx.run_all([
        "systemctl start postgresql".to_string(),
        "systemctl enable postgresql".to_string(),
    ])?;

    let db = &ctx.config.app.db;
    let dbuser = &ctx.config.app.dbuser;
    let dbpass = super::mysql::shell_quote(&ctx.config.app.dbpass);

    // `CREATE ROLE` and `CREATE DATABASE` via a here-doc piped to psql as the
    // postgres OS user.
    let sql = build_sql(db, dbuser, &dbpass);

    ctx.run(&format!(
        "echo '{sql}' | sudo -u postgres psql -v ON_ERROR_STOP=1"
    ))?;

    ctx.style.info("PostgreSQL configured successfully");
    Ok(())
}

/// Build the idempotent SQL that creates the role + database. Kept as a
/// standalone function for direct unit testing of the quoting and
/// idempotency clauses.
fn build_sql(db: &str, dbuser: &str, dbpass: &str) -> String {
    format!(
        "DO $$ BEGIN IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '{dbuser}') THEN \
            CREATE ROLE {dbuser} LOGIN PASSWORD '{dbpass}'; \
         END IF; END $$; \
         SELECT 'CREATE DATABASE {db} OWNER {dbuser}' WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = '{db}')\\gexec \
         GRANT ALL PRIVILEGES ON DATABASE {db} TO {dbuser};"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_creates_role_and_database() {
        let sql = build_sql("unit3d", "unit3d", "secretpass");
        assert!(sql.contains("CREATE ROLE unit3d LOGIN PASSWORD 'secretpass'"));
        assert!(sql.contains("CREATE DATABASE unit3d OWNER unit3d"));
        assert!(sql.contains("GRANT ALL PRIVILEGES ON DATABASE unit3d TO unit3d"));
    }

    #[test]
    fn sql_is_idempotent_guarded() {
        let sql = build_sql("unit3d", "unit3d", "secretpass");
        assert!(sql.contains("IF NOT EXISTS (SELECT 1 FROM pg_roles"));
        assert!(sql.contains("WHERE NOT EXISTS (SELECT 1 FROM pg_database"));
    }

    #[test]
    fn sql_quotes_dbpass_single_quotes() {
        // shell_quote strips single quotes, so the embedded SQL never breaks.
        let quoted = super::super::mysql::shell_quote("pa'ss'word");
        assert_eq!(quoted, "password");
        let sql = build_sql("unit3d", "unit3d", &quoted);
        assert!(sql.contains("PASSWORD 'password'"));
        assert!(!sql.contains("''"));
    }

    #[test]
    fn sql_uses_gexec_for_conditional_create() {
        let sql = build_sql("unit3d", "unit3d", "secretpass");
        assert!(sql.contains("\\gexec"));
    }

    #[test]
    fn configure_emits_psql_admin_command() {
        use crate::process::Exec;
        use crate::steps::Context;
        use std::sync::{Arc, Mutex};

        struct Recording(Arc<Mutex<Vec<String>>>);
        impl Exec for Recording {
            fn run(&self, cmd: &str) -> Result<std::process::Output> {
                self.0.lock().unwrap().push(cmd.to_string());
                Ok(std::process::Output {
                    status: std::os::unix::process::ExitStatusExt::from_raw(0),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            }
        }
        let cmds = Arc::new(Mutex::new(Vec::new()));
        let mut ctx = Context {
            config: crate::config::Config::default(),
            prompter: crate::io::Prompter::new(true),
            style: crate::io::Style,
            exec: Arc::new(Recording(cmds.clone())),
            dry_run: false,
            non_interactive: true,
            config_path: None,
        };
        ctx.config.app.db = "unit3d".to_string();
        ctx.config.app.dbuser = "unit3d".to_string();
        ctx.config.app.dbpass = "secretpass".to_string();
        configure(&mut ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(cmds.iter().any(|c| c.contains("sudo -u postgres psql")));
        assert!(
            cmds.iter()
                .any(|c| c.contains("CREATE ROLE unit3d LOGIN PASSWORD 'secretpass'"))
        );
    }
}
