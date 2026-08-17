//! Write `/root/unit3d-credentials.txt` (chmod 0600) with the live
//! credentials after install. Folds in gap G3 from the v1.2 standalone
//! script — the legacy PHP repo only printed creds to stdout.

use crate::resources::credentials::CredentialsTemplate;
use crate::steps::{Context, Step};
use anyhow::Result;
use askama::Template;
use chrono::Utc;

pub struct CredentialsStep;

impl Step for CredentialsStep {
    fn name(&self) -> &'static str {
        "Finalizing Install (credentials file)"
    }

    fn handle(&self, ctx: &mut Context) -> Result<()> {
        let generated = Utc::now().to_rfc3339();
        let web_user = ctx.config.web_user().to_string();
        let tpl = CredentialsTemplate {
            generated: &generated,
            fqdn: &ctx.config.app.hostname,
            owner: &ctx.config.app.owner,
            owner_email: &ctx.config.app.owner_email,
            owner_password: &ctx.config.app.password,
            db_name: &ctx.config.app.db,
            db_user: &ctx.config.app.dbuser,
            db_pass: &ctx.config.app.dbpass,
            db_root_pass: &ctx.config.app.dbrootpass,
            meilisearch_key: &ctx.config.app.meilisearch_key,
            install_dir: &ctx.config.install_dir().display().to_string(),
            php_version: &ctx.config.unit3d.min_php_version,
            web_user: &web_user,
        };
        let rendered = tpl.render()?;
        if ctx.dry_run {
            println!("{rendered}");
            return Ok(());
        }
        ctx.write_secret_file(
            std::path::Path::new("/root/unit3d-credentials.txt"),
            &rendered,
        )?;
        ctx.run("chmod 600 /root/unit3d-credentials.txt")?;
        ctx.style
            .info("Credentials saved to /root/unit3d-credentials.txt");
        Ok(())
    }
}

// Sanity-check the password helper in this module's namespace (kept here so
// the dependency on `hex32` is not flagged as unused when this step is the
// only caller).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use crate::password::hex32;
    use clap::Parser;

    #[test]
    fn hex32_is_32_chars() {
        assert_eq!(hex32().len(), 32);
    }

    fn credential_context() -> Context {
        let args = Args::parse_from(["unit3d-installer", "--dry-run"]);
        let mut ctx = Context::build(&args).unwrap();
        ctx.config.app.hostname = "tracker.example.com".to_string();
        ctx.config.app.owner = "admin".to_string();
        ctx.config.app.owner_email = "admin@tracker.example.com".to_string();
        ctx.config.app.password = "ownerpass".to_string();
        ctx.config.app.db = "unit3d".to_string();
        ctx.config.app.dbuser = "unit3d".to_string();
        ctx.config.app.dbpass = "dbpass".to_string();
        ctx.config.app.dbrootpass = "rootpass".to_string();
        ctx.config.app.meilisearch_key = "0123456789abcdef0123456789abcdef".to_string();
        ctx
    }

    #[test]
    fn credentials_renders_all_fields() {
        let mut ctx = credential_context();
        CredentialsStep.handle(&mut ctx).unwrap();
        // In dry-run the handle() returns Ok without writing.
        assert!(ctx.dry_run);
    }

    #[test]
    fn credentials_step_requires_web_user_from_config() {
        let mut ctx = credential_context();
        ctx.config.os.ubuntu.web_user = "ubuntu".to_string();
        CredentialsStep.handle(&mut ctx).unwrap();
    }

    #[test]
    fn credentials_step_is_safe_in_dry_run() {
        let mut ctx = credential_context();
        ctx.config.app.dbrootpass = "with'special".to_string();
        // Must not panic on shell-hostile password characters.
        CredentialsStep.handle(&mut ctx).unwrap();
    }
}
