//! Prerequisite apt packages + Node.js 24 LTS + Bun + laravel-echo-server +
//! UFW rules. Replaces `src/Installer/Prerequisites/Prerequisites.php` and
//! the inline apt calls from `ubuntu.sh`.

use crate::steps::{Context, Step};
use anyhow::Result;

pub struct PrerequisitesStep;

impl Step for PrerequisitesStep {
    fn name(&self) -> &'static str {
        "Prerequisites"
    }

    fn handle(&self, ctx: &mut Context) -> Result<()> {
        let software = ctx.config.os.ubuntu.software.clone();

        ctx.style.warning(
            "We are preparing to install software on your server. Please review and confirm!",
        );
        ctx.style.sep();
        for (pkg, desc) in &software.packages {
            println!("* '{pkg}': {desc}");
        }
        ctx.style.sep();
        if !ctx.prompter.confirm("Do you wish to continue?", true)? {
            anyhow::bail!("Aborted ...");
        }

        // Add PHP PPA, Node.js 20, Bun.
        ctx.run_all([
            "add-apt-repository -y ppa:ondrej/php".to_string(),
            "apt-get -qq update".to_string(),
            "curl -sL https://deb.nodesource.com/setup_24.x | sudo -E bash -".to_string(),
            "curl -fsSL https://bun.sh/install | bash".to_string(),
            "mv /root/.bun/bin/bun /usr/local/bin/ 2>/dev/null || true".to_string(),
            "chmod a+x /usr/local/bin/bun 2>/dev/null || true".to_string(),
            "npm install -g laravel-echo-server".to_string(),
        ])?;

        // Install all the listed apt packages.
        let pkgs: Vec<&str> = software.packages.keys().map(String::as_str).collect();
        let install_cmd = format!(
            "{} install -y {}",
            ctx.config.os.ubuntu.pkg_manager,
            pkgs.join(" ")
        );
        ctx.run(&install_cmd)?;

        // PHP extensions (php8.5-*) per the configured list.
        let exts = software.php_extensions.join(" ");
        ctx.run(&format!(
            "{} install -y {}",
            ctx.config.os.ubuntu.pkg_manager, exts
        ))?;

        // PECL Redis extension for PHP CLI.
        ctx.run("printf '\\n' | pecl install redis 2>/dev/null")?;

        // UFW: allow Nginx Full + the configured chat echo port (must match
        // the port used by the nginx proxy block and laravel-echo-server).
        let echo_port = ctx.config.app.echo_port;
        ctx.run_all([
            format!("ufw allow {echo_port}"),
            "ufw allow 'Nginx Full'".to_string(),
        ])?;

        ctx.style.info("Prerequisites installed");
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

    fn prereq_context() -> (Context, Arc<Mutex<Vec<String>>>) {
        let args = Args::parse_from(["unit3d-installer", "--non-interactive"]);
        let mut ctx = Context::build(&args).unwrap();
        ctx.config.app.echo_port = 9001;
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
    fn prerequisites_emits_core_setup_commands() {
        let (mut ctx, cmds) = prereq_context();
        PrerequisitesStep.handle(&mut ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(
            cmds.iter()
                .any(|c| c.contains("add-apt-repository -y ppa:ondrej/php"))
        );
        assert!(cmds.iter().any(|c| c.contains("setup_24.x")));
        assert!(cmds.iter().any(|c| c.contains("bun.sh/install")));
        assert!(
            cmds.iter()
                .any(|c| c.contains("npm install -g laravel-echo-server"))
        );
    }

    #[test]
    fn prerequisites_uses_configured_echo_port() {
        let (mut ctx, cmds) = prereq_context();
        PrerequisitesStep.handle(&mut ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(
            cmds.iter().any(|c| c == "ufw allow 9001"),
            "ufw must open configured echo port 9001"
        );
    }

    #[test]
    fn prerequisites_installs_all_packages_together() {
        let (mut ctx, cmds) = prereq_context();
        PrerequisitesStep.handle(&mut ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        let install = cmds
            .iter()
            .find(|c| c.starts_with("apt-get install -y"))
            .expect("apt install command");
        assert!(install.contains("nginx"));
        assert!(install.contains("redis-server"));
        assert!(install.contains("certbot"));
    }

    #[test]
    fn prerequisites_uses_configured_pkg_manager() {
        let args = Args::parse_from(["unit3d-installer", "--non-interactive"]);
        let (mut ctx, cmds) = prereq_context();
        ctx.config.os.ubuntu.pkg_manager = "apt".to_string();
        let _ = args;
        PrerequisitesStep.handle(&mut ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(
            cmds.iter().any(|c| c.starts_with("apt install -y")),
            "must use configured pkg manager"
        );
    }
}
