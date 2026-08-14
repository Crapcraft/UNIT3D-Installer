//! Nginx site configuration + Let's Encrypt SSL. Replaces
//! `src/Installer/Nginx/NginxSetup.php`. The written site file is the
//! full-featured template from gap analysis (G8-G12) with the `/socket.io`
//! proxy block on the configured echo port (G11) — the legacy repo's stub
//! was missing that block entirely.

use crate::resources::nginx::NginxTemplate;
use crate::steps::{Context, Step};
use anyhow::Result;
use askama::Template;

pub struct NginxSetupStep;

impl Step for NginxSetupStep {
    fn name(&self) -> &'static str {
        "Nginx Setup & Configurations"
    }

    fn handle(&self, ctx: &mut Context) -> Result<()> {
        let sites = ctx.config.os.ubuntu.nginx_sites_available_path.clone();
        let default = sites.join("default");

        let fqdn = ctx.config.app.hostname.clone();
        let install_dir = ctx.config.install_dir().display().to_string();
        let echo_port = ctx.config.app.echo_port;

        let tpl = NginxTemplate {
            fqdn: &fqdn,
            install_dir: &install_dir,
            echo_port,
            max_body: "256M",
        };
        let rendered = tpl.render()?;
        // Remove existing site file (route through exec so dry-run honors it).
        if !ctx.dry_run && default.exists() {
            std::fs::remove_file(&default)?;
        }
        ctx.write_file(&default, &rendered)?;
        ctx.style.info(&format!("wrote {}", default.display()));

        ctx.run_all([
            "rm -f /etc/nginx/sites-enabled/default".to_string(),
            "ln -sf /etc/nginx/sites-available/default /etc/nginx/sites-enabled/default"
                .to_string(),
            "nginx -t".to_string(),
            "systemctl restart nginx".to_string(),
            "systemctl enable nginx".to_string(),
            "ufw allow 'Nginx Full'".to_string(),
            format!("ufw allow {echo_port}"),
            "ufw --force enable".to_string(),
        ])?;

        // Certbot for Let's Encrypt — only when SSL is enabled.
        if ctx.config.app.ssl {
            let email = ctx.config.app.owner_email.clone();
            ctx.run(&format!(
                "certbot --redirect --nginx -n --agree-tos --email={email} -d {fqdn} -d www.{fqdn} --rsa-key-size 2048",
            ))?;
            // After SSL setup, re-cache Laravel config (URLs flip to https).
            let web_user = ctx.config.web_user().to_string();
            ctx.run(&format!(
                "sudo -u {web_user} php {install_dir}/artisan config:cache 2>/dev/null || true"
            ))
            .ok();
        }

        ctx.style.info("Nginx configured successfully");
        Ok(())
    }
}
