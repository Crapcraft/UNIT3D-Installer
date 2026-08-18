//! Prerequisite apt packages + Node.js 24 LTS + Bun + laravel-echo-server +
//! UFW rules. Replaces `src/Installer/Prerequisites/Prerequisites.php` and
//! the inline apt calls from `ubuntu.sh`.

use crate::steps::{Context, Step};
use anyhow::Result;

fn sanitize_php_extensions_for_version(version_id: &str, exts: Vec<String>) -> Vec<String> {
    let major: u32 = version_id
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut filtered = exts;
    if major >= 26 {
        filtered.retain(|e| !e.contains("opcache"));
    }
    filtered
}

/// Shell commands that add the PHP apt repository for the detected Ubuntu
/// release.
///
/// The ondrej/php PPA is being merged into packages.sury.org/php. For
/// Ubuntu 22.04 (Jammy) and 24.04 (Noble) the PPA still publishes packages,
/// but for 26.04 (Resolute) and newer the canonical source is
/// `https://packages.sury.org/php/` (adding `ppa:ondrej/php` there yields a
/// repo with no Release file, which aborts `apt-get update`).
fn php_repo_commands(_ctx: &Context) -> Vec<String> {
    let info = crate::system::detect().ok();
    let distro = info.as_ref().map(|i| i.distro);
    let version = info.as_ref().map(|i| i.version_id.as_str()).unwrap_or("");
    let major = version
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if matches!(distro, Some(crate::system::os_detect::Distro::Ubuntu)) && major >= 26 {
        vec![
            "rm -f /etc/apt/sources.list.d/ondrej-ubuntu-php-*.list /etc/apt/sources.list.d/ondrej-ubuntu-php-*.sources".to_string(),
            "curl -sSLo /tmp/debsuryorg-archive-keyring.deb https://packages.sury.org/debsuryorg-archive-keyring.deb".to_string(),
            "dpkg -i /tmp/debsuryorg-archive-keyring.deb".to_string(),
            "sh -c '. /etc/os-release; echo \"deb [signed-by=/usr/share/keyrings/debsuryorg-archive-keyring.gpg] https://packages.sury.org/php/ ${VERSION_CODENAME:-$(lsb_release -sc 2>/dev/null || echo bookworm)} main\" > /etc/apt/sources.list.d/php.list'".to_string(),
        ]
    } else if matches!(distro, Some(crate::system::os_detect::Distro::Debian)) {
        vec![
            "curl -sSLo /tmp/debsuryorg-archive-keyring.deb https://packages.sury.org/debsuryorg-archive-keyring.deb".to_string(),
            "dpkg -i /tmp/debsuryorg-archive-keyring.deb".to_string(),
            "sh -c '. /etc/os-release; echo \"deb [signed-by=/usr/share/keyrings/debsuryorg-archive-keyring.gpg] https://packages.sury.org/php/ ${VERSION_CODENAME:-bookworm} main\" > /etc/apt/sources.list.d/php.list'".to_string(),
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
        for (pkg, desc) in software.packages.iter().filter(|(pkg, _)| {
            if matches!(
                pkg.as_str(),
                "mysql-server" | "mariadb-server" | "postgresql"
            ) {
                pkg.as_str()
                    == match ctx.config.app.db_driver {
                        crate::config::DbDriver::Mysql => "mysql-server",
                        crate::config::DbDriver::MariaDb => "mariadb-server",
                        crate::config::DbDriver::Postgres => "postgresql",
                    }
            } else {
                true
            }
        }) {
            println!("* '{pkg}': {desc}");
        }
        ctx.style.sep();
        if !ctx.prompter.confirm("Do you wish to continue?", true)? {
            anyhow::bail!("Aborted ...");
        }

        // Determine the DB package to keep and the package list to install
        // (we need unzip before installing bun)
        let db_pkg = match ctx.config.app.db_driver {
            crate::config::DbDriver::Mysql => "mysql-server",
            crate::config::DbDriver::MariaDb => "mariadb-server",
            crate::config::DbDriver::Postgres => "postgresql",
        };

        let mut pkgs: Vec<String> = Vec::new();
        for pkg in software.packages.keys() {
            // Keep only the selected DB server package to avoid conflicts.
            if matches!(
                pkg.as_str(),
                "mysql-server" | "mariadb-server" | "postgresql"
            ) {
                if pkg == db_pkg {
                    pkgs.push(pkg.clone());
                }
            } else {
                pkgs.push(pkg.clone());
            }
        }

        let mut cmds = php_repo_commands(ctx);
        cmds.extend([
            "apt-get -qq update".to_string(),
            "curl -sL https://deb.nodesource.com/setup_24.x | sudo -E bash -".to_string(),
        ]);
        ctx.run_all(cmds)?;

        let install_cmd = format!(
            "{} install -y {}",
            ctx.config.os.ubuntu.pkg_manager,
            pkgs.join(" ")
        );
        ctx.run(&install_cmd)?;

        ctx.run_all([
            "curl -fsSL https://bun.sh/install | bash".to_string(),
            "mv /root/.bun/bin/bun /usr/local/bin/ 2>/dev/null || true".to_string(),
            "chmod a+x /usr/local/bin/bun 2>/dev/null || true".to_string(),
            "npm install -g laravel-echo-server".to_string(),
        ])?;

        let version_id = crate::system::detect()
            .map(|info| info.version_id)
            .unwrap_or_default();
        let exts =
            sanitize_php_extensions_for_version(&version_id, software.php_extensions.clone())
                .join(" ");
        if !exts.is_empty() {
            ctx.run(&format!(
                "{} install -y {}",
                ctx.config.os.ubuntu.pkg_manager, exts
            ))?;
        }

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
    fn php_extensions_drop_opcache_on_26_and_newer() {
        let exts = sanitize_php_extensions_for_version(
            "26.04",
            crate::config::SoftwareSection::default().php_extensions,
        );
        assert!(!exts.iter().any(|e| e.contains("opcache")));

        let exts = sanitize_php_extensions_for_version(
            "24.04",
            crate::config::SoftwareSection::default().php_extensions,
        );
        assert!(exts.iter().any(|e| e.contains("opcache")));
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
        // Stale ondrej PPA sources from an earlier run must be removed.
        assert!(cmds.iter().any(|c| c.contains("ondrej-ubuntu-php-*.list")));
        // The PPA must never be used on 26.04.
        assert!(!cmds.iter().any(|c| c.contains("ppa:ondrej/php")));
    }

    #[test]
    fn php_repo_uses_sury_for_debian() {
        let cmds = php_repo_commands_for_version("12");
        assert!(
            cmds.iter()
                .any(|c| c.contains("packages.sury.org/debsuryorg-archive-keyring.deb"))
        );
        assert!(cmds.iter().any(|c| c.contains("packages.sury.org/php/")));
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
                "rm -f /etc/apt/sources.list.d/ondrej-ubuntu-php-*.list /etc/apt/sources.list.d/ondrej-ubuntu-php-*.sources"
                    .to_string(),
                "curl -sSLo /tmp/debsuryorg-archive-keyring.deb https://packages.sury.org/debsuryorg-archive-keyring.deb"
                    .to_string(),
                "dpkg -i /tmp/debsuryorg-archive-keyring.deb".to_string(),
                "sh -c 'echo \"deb [signed-by=/usr/share/keyrings/debsuryorg-archive-keyring.gpg] https://packages.sury.org/php/ $(lsb_release -sc) main\" > /etc/apt/sources.list.d/php.list'"
                    .to_string(),
            ]
        } else if version_id == "12" || version_id.starts_with("12.") {
            vec![
                "curl -sSLo /tmp/debsuryorg-archive-keyring.deb https://packages.sury.org/debsuryorg-archive-keyring.deb"
                    .to_string(),
                "dpkg -i /tmp/debsuryorg-archive-keyring.deb".to_string(),
                "sh -c '. /etc/os-release; echo \"deb [signed-by=/usr/share/keyrings/debsuryorg-archive-keyring.gpg] https://packages.sury.org/php/ ${VERSION_CODENAME:-bookworm} main\" > /etc/apt/sources.list.d/php.list'"
                    .to_string(),
            ]
        } else {
            vec!["add-apt-repository -y ppa:ondrej/php".to_string()]
        }
    }
}
