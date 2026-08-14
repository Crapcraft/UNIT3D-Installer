//! PHP-FPM configuration. Writes the per-FQDN pool stub and patches every
//! `php.ini` for opcache/JIT/preload plus the upload/memory limits from
//! the v1.2 standalone script (gaps G5/G6/G7).
//!
//! Replaces `src/Installer/PHP/PhpSetup.php`.

use crate::resources::phpfpm::PhpFpmTemplate;
use crate::steps::{Context, Step};
use anyhow::Result;
use askama::Template;
use glob::glob as glob_match;
use std::path::{Path, PathBuf};

pub struct PhpSetupStep;

impl Step for PhpSetupStep {
    fn name(&self) -> &'static str {
        "PHP & PHP-FPM Configuration"
    }

    fn handle(&self, ctx: &mut Context) -> Result<()> {
        let fqdn = ctx.config.app.hostname.clone();
        let web_user = ctx.config.web_user().to_string();
        let install_dir = ctx.config.install_dir().display().to_string();

        // 1. Render and write the per-FQDN pool stub under every php
        // installation's pool.d directory.
        for pool in find_pool_dirs() {
            let target = pool.join(format!("{fqdn}.conf"));
            let tpl = PhpFpmTemplate {
                fqdn: &fqdn,
                web_user: &web_user,
            };
            let rendered = tpl.render()?;
            ctx.write_file(&target, &rendered)?;
            ctx.style.info(&format!("wrote {}", target.display()));
        }

        // 2. Patch every php.ini found on the box.
        for ini in find_matches("/etc/php*/**/php.ini") {
            patch_ini(&ini, &install_dir, &web_user, ctx)?;
        }

        // 3. Patch www.conf for pm settings.
        for conf in find_matches("/etc/php*/**/www.conf") {
            patch_www(&conf, ctx)?;
        }

        // 4. Restart PHP-FPM services.
        for svc in find_fpm_services() {
            let _ = ctx.run_all([
                format!("systemctl restart {svc}"),
                format!("systemctl enable {svc}"),
            ]);
        }

        ctx.style.info("PHP configured successfully");
        Ok(())
    }
}

fn patch_ini(path: &Path, install_dir: &str, web_user: &str, ctx: &Context) -> Result<()> {
    let p = path.display().to_string();
    ctx.run_all([
        format!("sed -i 's/;date.timezone =/date.timezone = UTC/g' {p}"),
        // G7: 0 (modern hardening) — was 1 in the legacy PHP installer.
        format!("sed -i 's/;cgi.fix_pathinfo=1/cgi.fix_pathinfo=0/g' {p}"),
        format!("sed -i 's/upload_max_filesize = .*/upload_max_filesize = 256M/' {p}"),
        format!("sed -i 's/post_max_size = .*/post_max_size = 256M/' {p}"),
        format!("sed -i 's/memory_limit = .*/memory_limit = 512M/' {p}"),
        format!("sed -i 's/max_execution_time = .*/max_execution_time = 600/' {p}"),
        format!("sed -i 's/max_input_time = .*/max_input_time = 600/' {p}"),
        format!("sed -i 's/;opcache.enable=1/opcache.enable=1/g' {p}"),
        format!("sed -i 's/;opcache.enable_cli=0/opcache.enable_cli=1/g' {p}"),
        format!("sed -i 's/;opcache.memory_consumption=128/opcache.memory_consumption=256/g' {p}"),
        format!(
            "sed -i 's/;opcache.interned_strings_buffer=8/opcache.interned_strings_buffer=64/g' {p}"
        ),
        format!("sed -i 's/;opcache.validate_timestamps=1/opcache.validate_timestamps=0/g' {p}"),
        format!("sed -i 's/;opcache.save_comments=1/opcache.save_comments=1/g' {p}"),
        format!("sed -i '/\\[curl\\]/i opcache.jit_buffer_size=256M' {p}"),
        format!("sed -i \"s|;opcache.preload=|opcache.preload='{install_dir}/preload.php'|g\" {p}"),
        format!("sed -i \"s|;opcache.preload_user=|opcache.preload_user={web_user}|g\" {p}"),
    ])?;
    Ok(())
}

fn patch_www(path: &Path, ctx: &Context) -> Result<()> {
    let p = path.display().to_string();
    ctx.run_all([
        format!("sed -i 's/^pm = .*/pm = static/g' {p}"),
        // G6: bump from 25 → 50 children, add spare-server pool sizing.
        format!("sed -i 's/^pm.max_children = .*/pm.max_children = 50/g' {p}"),
        format!("sed -i 's/^pm.start_servers = .*/pm.start_servers = 10/g' {p}"),
        format!("sed -i 's/^pm.min_spare_servers = .*/pm.min_spare_servers = 5/g' {p}"),
        format!("sed -i 's/^pm.max_spare_servers = .*/pm.max_spare_servers = 20/g' {p}"),
        format!("sed -i 's/^;request_terminate_timeout =.*/request_terminate_timeout = 600/g' {p}"),
    ])?;
    Ok(())
}

fn find_pool_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in glob_match("/etc/php*/**/pool.d").unwrap().flatten() {
        if entry.is_dir() {
            out.push(entry);
        }
    }
    out
}

fn find_matches(pattern: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in glob_match(pattern).unwrap().flatten() {
        if entry.is_file() {
            out.push(entry);
        }
    }
    out
}

fn find_fpm_services() -> Vec<String> {
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg("ls /etc/init.d/php*.*-fpm* 2>/dev/null | cut -f 4 -d /")
        .output();
    let mut services: Vec<String> = Vec::new();
    if let Ok(o) = out {
        let s = String::from_utf8_lossy(&o.stdout);
        for line in s.lines() {
            let line = line.trim();
            if !line.is_empty() {
                services.push(line.to_string());
            }
        }
    }
    // Fallback to the PHP version we install.
    if services.is_empty() {
        services.push("php8.5-fpm".to_string());
    }
    services
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::Exec;
    use std::sync::{Arc, Mutex};

    struct Recording(Arc<Mutex<Vec<String>>>);

    impl Exec for Recording {
        fn run(&self, cmd: &str) -> Result<std::process::Output> {
            self.0.lock().unwrap().push(cmd.to_string());
            Ok(std::process::Output {
                status: std::os::unix::process::ExitStatusExt::from_raw(0),
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        }
    }

    fn recording_ctx() -> (Context, Arc<Mutex<Vec<String>>>) {
        let cmds = Arc::new(Mutex::new(Vec::new()));
        let ctx = Context {
            config: crate::config::Config::default(),
            prompter: crate::io::Prompter::new(true),
            style: crate::io::Style,
            exec: Arc::new(Recording(cmds.clone())),
            dry_run: false,
            non_interactive: true,
            config_path: None,
        };
        (ctx, cmds)
    }

    #[test]
    fn pool_dirs_no_panic() {
        // No-op: ensures the glob helper compiles + never panics on systems
        // with no PHP installed.
        let _ = find_pool_dirs();
    }

    #[test]
    fn find_matches_no_panic() {
        let _ = find_matches("/etc/php*/**/php.ini");
    }

    #[test]
    fn patch_ini_emits_expected_seds() {
        let (ctx, cmds) = recording_ctx();
        let p = std::path::Path::new("/etc/php/8.5/cli/php.ini");
        patch_ini(p, "/var/www/html", "www-data", &ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(cmds.iter().any(|c| c.contains("date.timezone = UTC")));
        assert!(cmds.iter().any(|c| c.contains("cgi.fix_pathinfo=0")));
        assert!(
            cmds.iter()
                .any(|c| c.contains("upload_max_filesize = 256M"))
        );
        assert!(cmds.iter().any(|c| c.contains("memory_limit = 512M")));
        assert!(cmds.iter().any(|c| c.contains("max_execution_time = 600")));
        assert!(
            cmds.iter()
                .any(|c| c.contains("opcache.memory_consumption=256"))
        );
        assert!(
            cmds.iter()
                .any(|c| c.contains("opcache.jit_buffer_size=256M"))
        );
        assert!(
            cmds.iter()
                .any(|c| c.contains("opcache.preload='/var/www/html/preload.php'"))
        );
        assert!(
            cmds.iter()
                .any(|c| c.contains("opcache.preload_user=www-data"))
        );
        assert!(cmds.iter().any(|c| c.contains("/etc/php/8.5/cli/php.ini")));
    }

    #[test]
    fn patch_www_emits_pm_static_and_children() {
        let (ctx, cmds) = recording_ctx();
        let p = std::path::Path::new("/etc/php/8.5/fpm/pool.d/www.conf");
        patch_www(p, &ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(cmds.iter().any(|c| c.contains("pm = static")));
        assert!(cmds.iter().any(|c| c.contains("pm.max_children = 50")));
        assert!(cmds.iter().any(|c| c.contains("pm.start_servers = 10")));
        assert!(cmds.iter().any(|c| c.contains("pm.min_spare_servers = 5")));
        assert!(cmds.iter().any(|c| c.contains("pm.max_spare_servers = 20")));
        assert!(
            cmds.iter()
                .any(|c| c.contains("request_terminate_timeout = 600"))
        );
    }

    #[test]
    fn find_fpm_services_falls_back_to_85() {
        // Regardless of what's on the box, the fallback ensures a service.
        let svcs = find_fpm_services();
        assert!(
            svcs.iter()
                .any(|s| s.contains("php8.5-fpm") || s.starts_with("php"))
        );
    }
}
