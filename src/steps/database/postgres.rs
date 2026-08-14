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
    let sql = format!(
        "DO $$ BEGIN IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '{dbuser}') THEN \
            CREATE ROLE {dbuser} LOGIN PASSWORD '{dbpass}'; \
         END IF; END $$; \
         SELECT 'CREATE DATABASE {db} OWNER {dbuser}' WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = '{db}')\\gexec \
         GRANT ALL PRIVILEGES ON DATABASE {db} TO {dbuser};"
    );

    ctx.run(&format!(
        "echo '{sql}' | sudo -u postgres psql -v ON_ERROR_STOP=1"
    ))?;

    ctx.style.info("PostgreSQL configured successfully");
    Ok(())
}
