//! MariaDB driver — forwards to the shared MySQL logic with `MariaDb`
//! flavor. Kept as a thin module so the dispatch in
//! [`super::DatabaseStep`] stays readable.

use crate::steps::Context;
use anyhow::Result;

pub fn configure(ctx: &mut Context) -> Result<()> {
    super::mysql::configure(ctx)
}
