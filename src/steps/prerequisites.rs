//! Prerequisite apt packages + Node.js 24 LTS + Bun + laravel-echo-server +
//! UFW rules. Replaces `src/Installer/Prerequisites/Prerequisites.php` and
//! the inline apt calls from `ubuntu.sh`.

use crate::steps::{Context, Step};
use anyhow::Result;

/// Shell commands that add the PHP apt repository for the detected Ubuntu
/// release.
///
/// The ondrej/php PPA is being merged into packages.sury.org/php. For
/// Ubuntu 22.04 (Jammy) and 24.04 (Noble) the PPA still publishes packages,
/// but for 26.04 (Resolute) and newer the canonical source is
/// `https://packages.sury.org/php/` (adding `ppa:ondrej/php` there yields a
/// repo with no Release file, which aborts `apt-get update`).
fn php_repo_commands(ctx: &Context) -> Vec<String> {
    let version_id = if ctx.dry_run {
        // Don't read the live box during a dry run; preview the legacy PPA
        // path so the output is deterministic across machines.
        String::new()
    } else {
        crate::system::detect()
            .map(|info| info.version_id)
            .unwrap_or_default()
    };
    let major: u32 = version_id
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if major >= 26 {
        vec![
            "curl -sSLo /tmp/debsuryorg-archive-keyring.deb https://packages.sury.org/debsuryorg-archive-keyring.deb"
                .to_string(),
            "dpkg -i /tmp/debsuryorg-archive-keyring.deb".to_string(),
            "sh -c 'echo \"deb [signed-by=/usr/share/keyrings/debsuryorg-archive-keyring.gpg] https://packages.sury.org/php/ $(lsb_release -sc) main\" > /etc/apt/sources.list.d/php.list'"
                .to_string(),
        ]
    } else {
        vec!["add-apt-repository -y ppa:ondrej/php".to_string()]
    }
}

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

        // Add the PHP repository, Node.js 24, Bun. The ondrej/php PPA only
        // publishes packages for Jammy (22.04) and Noble (24.04); for Ubuntu
        // 26.04 (Resolute) and newer, PHP packages are canonical at
        // packages.sury.org/php/ (the PPA is being merged into it).
        let mut cmds = php_repo_commands(ctx);
        cmds.extend([
            "apt-get -qq update".to_string(),
            "curl -sL https://deb.nodesource.com/setup_24.x | sudo -E bash -".to_string(),
            "curl -fsSL https://bun.sh/install | bash".to_string(),
            "mv /root/.bun/bin/bun /usr/local/bin/ 2>/dev/null || true".to_string(),
            "chmod a+x /usr/local/bin/bun 2>/dev/null || true".to_string(),
            "npm install -g laravel-echo-server".to_string(),
        ]);
        ctx.run_all(cmds)?;

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
        let args = Args::parse_from(["unit3d-installer", "--non-interactive", "--dry-run"]);
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

    #[test]
    fn php_repo_legacy_ppa_for_24_and_below() {
        // Jammy/Noble still get the legacy PPA.
        assert_eq!(
            php_repo_commands_for_version("24.04"),
            vec!["add-apt-repository -y ppa:ondrej/php"]
        );
        assert_eq!(
            php_repo_commands_for_version("22.04.3"),
            vec!["add-apt-repository -y ppa:ondrej/php"]
        );
        // Unknown/empty version falls back to the legacy PPA too.
        assert_eq!(
            php_repo_commands_for_version(""),
            vec!["add-apt-repository -y ppa:ondrej/php"]
        );
    }

    #[test]
    fn php_repo_uses_sury_for_26_and_newer() {
        let cmds = php_repo_commands_for_version("26.04");
        assert!(
            cmds.iter()
                .any(|c| c.contains("packages.sury.org/debsuryorg-archive-keyring.deb"))
        );
        assert!(
            cmds.iter()
                .any(|c| c.contains("dpkg -i /tmp/debsuryorg-archive-keyring.deb"))
        );
        assert!(cmds.iter().any(|c| c.contains("packages.sury.org/php/")
            && c.contains("signed-by=/usr/share/keyrings/debsuryorg-archive-keyring.gpg")));
        // The PPA must never be used on 26.04.
        assert!(!cmds.iter().any(|c| c.contains("ppa:ondrej/php")));
    }

    fn php_repo_commands_for_version(version_id: &str) -> Vec<String> {
        let major: u32 = version_id
            .split('.')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        if major >= 26 {
            vec![
                "curl -sSLo /tmp/debsuryorg-archive-keyring.deb https://packages.sury.org/debsuryorg-archive-keyring.deb"
                    .to_string(),
                "dpkg -i /tmp/debsuryorg-archive-keyring.deb".to_string(),
                "sh -c 'echo \"deb [signed-by=/usr/share/keyrings/debsuryorg-archive-keyring.gpg] https://packages.sury.org/php/ $(lsb_release -sc) main\" > /etc/apt/sources.list.d/php.list'"
                    .to_string(),
            ]
        } else {
            vec!["add-apt-repository -y ppa:ondrej/php".to_string()]
        }
    }
}
