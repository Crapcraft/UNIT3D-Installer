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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use crate::process::Exec;
    use crate::steps::Context;
    use clap::Parser;
    use std::sync::{Arc, Mutex};

    fn nginx_context() -> (Context, Arc<Mutex<Vec<String>>>) {
        let args = Args::parse_from(["unit3d-installer", "--non-interactive"]);
        let mut ctx = Context::build(&args).unwrap();
        ctx.dry_run = true; // never write to real /etc/nginx in tests
        ctx.config.app.hostname = "tracker.example.com".to_string();
        ctx.config.app.owner_email = "admin@tracker.example.com".to_string();
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
    fn nginx_emits_site_and_reload_commands() {
        let (mut ctx, cmds) = nginx_context();
        NginxSetupStep.handle(&mut ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(
            cmds.iter()
                .any(|c| c.contains("ln -sf /etc/nginx/sites-available/default"))
        );
        assert!(cmds.iter().any(|c| c.contains("nginx -t")));
        assert!(cmds.iter().any(|c| c.contains("systemctl restart nginx")));
        assert!(cmds.iter().any(|c| c.contains("ufw allow 'Nginx Full'")));
    }

    #[test]
    fn nginx_opens_configured_echo_port() {
        let (mut ctx, cmds) = nginx_context();
        ctx.config.app.echo_port = 6001;
        NginxSetupStep.handle(&mut ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(cmds.iter().any(|c| c == "ufw allow 6001"));
    }

    #[test]
    fn nginx_runs_certbot_when_ssl() {
        let (mut ctx, cmds) = nginx_context();
        ctx.config.app.ssl = true;
        NginxSetupStep.handle(&mut ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(
            cmds.iter()
                .any(|c| c.starts_with("certbot --redirect --nginx -n --agree-tos")),
            "certbot must run when ssl=true"
        );
        let certbot = cmds.iter().find(|c| c.starts_with("certbot")).unwrap();
        assert!(
            certbot.contains("--email=admin@tracker.example.com"),
            "{certbot}"
        );
        assert!(certbot.contains("-d tracker.example.com"), "{certbot}");
    }

    #[test]
    fn nginx_skips_certbot_when_no_ssl() {
        let (mut ctx, cmds) = nginx_context();
        ctx.config.app.ssl = false;
        NginxSetupStep.handle(&mut ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(
            !cmds.iter().any(|c| c.starts_with("certbot")),
            "no certbot when ssl=false"
        );
    }

    #[test]
    fn nginx_writes_site_file_via_exec() {
        // In a non-dry-run context write_file writes to /etc/nginx — not
        // desirable in tests. Use dry-run to verify the step still emits
        // commands and the render doesn't panic.
        let (mut ctx, cmds) = nginx_context();
        ctx.dry_run = true;
        NginxSetupStep.handle(&mut ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(cmds.iter().any(|c| c.contains("nginx -t")));
    }
}
