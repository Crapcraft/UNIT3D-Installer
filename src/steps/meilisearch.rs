//! Install + configure Meilisearch. Replaces `src/Installer/UNIT3D/
//! MeilisearchSetup.php` and folds in gap G16 (`scout:import
//! App\Models\Torrent`).

use crate::resources::meilisearch_toml::MeilisearchTomlTemplate;
use crate::resources::meilisearch_unit::MeilisearchUnitTemplate;
use crate::steps::{Context, Step};
use anyhow::Result;
use askama::Template;

pub struct MeilisearchSetupStep;

impl Step for MeilisearchSetupStep {
    fn name(&self) -> &'static str {
        "Meilisearch Setup & Configuration"
    }

    fn handle(&self, ctx: &mut Context) -> Result<()> {
        let web_user = ctx.config.web_user().to_string();
        let master_key = ctx.config.app.meilisearch_key.clone();
        let install_dir = ctx.config.install_dir().display().to_string();

        ctx.style.section("Installing and Configuring Meilisearch");

        ctx.run_all([
            "curl -L https://install.meilisearch.com | sh".to_string(),
            "mv ./meilisearch /usr/local/bin/".to_string(),
            "chmod +x /usr/local/bin/meilisearch".to_string(),
            "mkdir -p /var/lib/meilisearch/data /var/lib/meilisearch/dumps /var/lib/meilisearch/snapshots".to_string(),
            format!("chown -R {web_user}:{web_user} /var/lib/meilisearch"),
            "chmod -R 750 /var/lib/meilisearch".to_string(),
        ])?;

        // /etc/meilisearch.toml
        let toml_tpl = MeilisearchTomlTemplate {
            master_key: &master_key,
            db_path: "/var/lib/meilisearch/data",
            dump_dir: "/var/lib/meilisearch/dumps",
            snapshot_dir: "/var/lib/meilisearch/snapshots",
        };
        let toml_rendered = toml_tpl.render()?;
        ctx.write_file(
            std::path::Path::new("/etc/meilisearch.toml"),
            &toml_rendered,
        )?;

        // systemd unit
        let unit_tpl = MeilisearchUnitTemplate {
            web_user: &web_user,
        };
        let unit_rendered = unit_tpl.render()?;
        ctx.write_file(
            std::path::Path::new("/etc/systemd/system/meilisearch.service"),
            &unit_rendered,
        )?;
        ctx.run_all([
            "systemctl daemon-reload".to_string(),
            "systemctl enable meilisearch".to_string(),
            "systemctl start meilisearch".to_string(),
        ])?;

        ctx.style.section("Syncing Meilisearch Indexes");
        // G16: import the Torrent model in addition to syncing index
        // settings.
        let cmds = [
            "php artisan scout:sync-index-settings",
            "php artisan scout:import \"App\\Models\\Torrent\"",
        ];
        for cmd in cmds {
            let s = format!("sudo -u {web_user} bash -c 'cd {install_dir} && {cmd}'");
            let _ = ctx.run(&s);
        }

        ctx.style.info("Meilisearch configured successfully");
        Ok(())
    }
}
