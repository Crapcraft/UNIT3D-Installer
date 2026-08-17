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

    // `CREATE ROLE` and `CREATE DATABASE` via a here-doc piped to psql as the
    // postgres OS user. All interpolated identifiers/passwords are scrubbed
    // so a hostile config value can't escape the `echo '...'` shell string.
    let sql = build_sql(
        &ctx.config.app.db,
        &ctx.config.app.dbuser,
        &ctx.config.app.dbpass,
    );

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
    let db = super::mysql::shell_quote(db);
    let dbuser = super::mysql::shell_quote(dbuser);
    let dbpass = super::mysql::shell_quote(dbpass);
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
    fn sql_scrubs_injection_in_identifiers() {
        // db/dbuser with quotes, semicolons and shell chars must be scrubbed
        // so they can never escape the `echo '...'` shell string or break
        // out of the SQL statement.
        let sql = build_sql("db;DROP TABLE x", "u'name", "p;ass");
        // Identifiers must be scrubbed (the `;` and quotes stripped).
        assert!(sql.contains("CREATE ROLE uname"), "got: {sql}");
        assert!(sql.contains("CREATE DATABASE dbDROP TABLE x"), "got: {sql}");
        assert!(sql.contains("PASSWORD 'pass'"), "got: {sql}");
        // No user-controlled value may contain a quote that could break the
        // surrounding `echo '...'` shell string. The SQL's own structural
        // quotes (rolname='uname', \gexec trick) are fine and expected.
        for bad in ["u'name", "p;ass", "db;DROP"] {
            assert!(!sql.contains(bad), "{bad} leaked into SQL: {sql}");
        }
        // The full command is `echo '{sql}' | sudo -u postgres psql ...`;
        // every single-quote in the SQL must be a balanced structural pair,
        // never an unbalanced breaker. Count: the echo wrapper adds 2.
        let echo = format!("echo '{sql}'");
        assert_eq!(
            echo.matches('\'').count() % 2,
            0,
            "unbalanced quotes: {echo}"
        );
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
