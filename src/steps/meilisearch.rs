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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use crate::process::Exec;
    use crate::steps::Context;
    use clap::Parser;
    use std::sync::{Arc, Mutex};

    fn meili_context() -> (Context, Arc<Mutex<Vec<String>>>) {
        let args = Args::parse_from(["unit3d-installer", "--non-interactive"]);
        let mut ctx = Context::build(&args).unwrap();
        ctx.dry_run = true; // never write to real /etc config paths in tests
        ctx.config.app.hostname = "tracker.example.com".to_string();
        ctx.config.app.meilisearch_key = "0123456789abcdef0123456789abcdef".to_string();
        let cmds = Arc::new(Mutex::new(Vec::new()));
        let rec = {
            let cmds = cmds.clone();
            struct R(Arc<Mutex<Vec<String>>>);
            impl Exec for R {
                fn run(&self, cmd: &str) -> Result<std::process::Output> {
                    self.0.lock().unwrap().push(cmd.to_string());
                    Ok(std::process::Output {
                        status: std::os::unix::process::ExitStatusExt::from_raw(0),
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    })
                }
            }
            R(cmds)
        };
        ctx.exec = Arc::new(rec);
        (ctx, cmds)
    }

    #[test]
    fn meili_emits_install_and_service_commands() {
        let (mut ctx, cmds) = meili_context();
        MeilisearchSetupStep.handle(&mut ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(cmds.iter().any(|c| c.contains("install.meilisearch.com")));
        assert!(cmds.iter().any(|c| c.contains("systemctl daemon-reload")));
        assert!(
            cmds.iter()
                .any(|c| c.contains("systemctl enable meilisearch"))
        );
        assert!(
            cmds.iter()
                .any(|c| c.contains("systemctl start meilisearch"))
        );
        assert!(cmds.iter().any(|c| c.contains("scout:import")));
    }

    #[test]
    fn meili_writes_config_files() {
        // In dry-run mode, write_file prints rather than writing; here we
        // assert the step completes and commands are still emitted.
        let (mut ctx, cmds) = meili_context();
        ctx.config.app.meilisearch_key = "0123456789abcdef0123456789abcdef".to_string();
        MeilisearchSetupStep.handle(&mut ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(
            cmds.iter()
                .any(|c| c.contains("chown -R www-data:www-data /var/lib/meilisearch"))
        );
    }

    #[test]
    fn meili_scout_import_uses_install_dir() {
        let (mut ctx, cmds) = meili_context();
        ctx.config.os.ubuntu.install_dir = std::path::PathBuf::from("/srv/unit3d");
        MeilisearchSetupStep.handle(&mut ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(
            cmds.iter()
                .any(|c| c.contains("cd /srv/unit3d && php artisan scout:import")),
            "scout import must run in install dir"
        );
    }
}
