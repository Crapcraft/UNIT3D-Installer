//! Database provisioning. Dispatches to MySQL / MariaDB / PostgreSQL
//! installers via the `db_driver` configured in [`Config`].
//!
//! Replaces `src/Installer/Database/{DatabaseSetup,MySqlSetup,
//! MariaDbSetup}.php` plus the new PostgreSQL support selected in the
//! plan.

pub mod mariadb;
pub mod mysql;
pub mod postgres;

use crate::config::DbDriver;
use crate::steps::{Context, Step};
use anyhow::Result;

pub struct DatabaseStep;

impl Step for DatabaseStep {
    fn name(&self) -> &'static str {
        "Configuring & Securing Database"
    }

    fn handle(&self, ctx: &mut Context) -> Result<()> {
        match ctx.config.app.db_driver {
            DbDriver::Mysql => mysql::configure(ctx),
            DbDriver::MariaDb => mariadb::configure(ctx),
            DbDriver::Postgres => postgres::configure(ctx),
        }
    }
}
