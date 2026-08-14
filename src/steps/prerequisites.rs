//! Prerequisite apt packages + Node.js 20 + Bun + laravel-echo-server +
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
            "curl -sL https://deb.nodesource.com/setup_20.x | sudo -E bash -".to_string(),
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

        // UFW: allow Nginx Full + the chat echo port.
        ctx.run_all([
            "ufw allow 8443".to_string(),
            "ufw allow 'Nginx Full'".to_string(),
        ])?;

        ctx.style.info("Prerequisites installed");
        Ok(())
    }
}
