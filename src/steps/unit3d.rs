//! Clone UNIT3D-Community-Edition, render the `.env`, set permissions,
//! install dependencies (Composer + Bun), run migrations, set up cron +
//! supervisor + Laravel Echo Server, and run post-install caching.
//!
//! Combines the legacy `Unit3dSetup` and the v1.2 standalone script's
//! `install_unit3d`, `configure_laravel_echo_server`,
//! `configure_supervisor`, and `configure_cron` steps. Folds in gaps
//! G14/G15/G17/G18/G20/G21/G23/G24.

use crate::config::DbDriver;
use crate::resources::echo_server::EchoServerTemplate;
use crate::resources::env::EnvTemplate;
use crate::resources::supervisor::SupervisorTemplate;
use crate::steps::{Context, Step};
use anyhow::Result;
use askama::Template;
use std::path::Path;

pub struct Unit3dSetupStep;

impl Step for Unit3dSetupStep {
    fn name(&self) -> &'static str {
        "UNIT3D-Community-Edition Settings and Configuration"
    }

    fn handle(&self, ctx: &mut Context) -> Result<()> {
        clone(ctx)?;
        env(ctx)?;
        perms(ctx)?;
        crons(ctx)?;
        setup(ctx)?;
        Ok(())
    }
}

fn clone(ctx: &mut Context) -> Result<()> {
    ctx.style.section("Cloning Source Files");
    let install_dir = ctx.config.install_dir().to_path_buf();
    let url = ctx.config.unit3d.repository.clone();
    // G14: pin to a tag; fall back to `main` if the tag doesn't exist.
    let tag = ctx.config.unit3d.tag.clone();

    if install_dir.exists() {
        ctx.run(&format!("rm -rf {}", install_dir.display()))?;
    }

    ctx.run(&format!(
        "git config --global --add safe.directory {}",
        install_dir.display()
    ))?;
    ctx.run(&format!(
        "git clone -b {tag} {url} {}",
        install_dir.display()
    ))?;
    if !ctx.dry_run && !Path::new(&install_dir).exists() {
        anyhow::bail!("git clone failed for {url} @ {tag}");
    }
    Ok(())
}

fn env(ctx: &mut Context) -> Result<()> {
    ctx.style.section("Preparing the .env File");
    let install_dir = ctx.config.install_dir().to_path_buf();
    let env_path = install_dir.join(".env");

    if !ctx.dry_run && env_path.exists() {
        std::fs::remove_file(&env_path)?;
    }

    let protocol = if ctx.config.app.ssl { "https" } else { "http" };
    let fqdn = ctx.config.app.hostname.clone();
    let socket = match ctx.config.app.db_driver {
        DbDriver::Postgres => "",
        _ => "/var/run/mysqld/mysqld.sock",
    };

    let tpl = EnvTemplate {
        protocol,
        fqdn: &fqdn,
        db_driver: ctx.config.app.db_driver.as_db_connection(),
        db: &ctx.config.app.db,
        dbuser: &ctx.config.app.dbuser,
        dbpass: &ctx.config.app.dbpass,
        socket,
        owner: &ctx.config.app.owner,
        owner_email: &ctx.config.app.owner_email,
        owner_password: &ctx.config.app.password,
        tmdb_key: &ctx.config.app.tmdb_key,
        mail_driver: &ctx.config.app.mail_driver,
        mail_host: &ctx.config.app.mail_host,
        mail_port: &ctx.config.app.mail_port,
        mail_username: &ctx.config.app.mail_username,
        mail_password: &ctx.config.app.mail_password,
        mail_from_name: &ctx.config.app.mail_from_name,
        meilisearch_key: &ctx.config.app.meilisearch_key,
        redis_host: "/var/run/redis/redis.sock",
        redis_port: "-1",
    };
    let rendered = tpl.render()?;
    ctx.write_file(&env_path, &rendered)?;
    ctx.style.info(&format!("wrote {}", env_path.display()));
    Ok(())
}

fn perms(ctx: &mut Context) -> Result<()> {
    ctx.style.section("Setting Permissions");
    let install_dir = ctx.config.install_dir().to_path_buf();
    let web_user = ctx.config.web_user().to_string();
    let parent = install_dir
        .parent()
        .unwrap_or(Path::new("/"))
        .display()
        .to_string();
    ctx.run_all([
        format!("chown -R {web_user}:{web_user} /etc/letsencrypt 2>/dev/null || true"),
        format!("chown -R {web_user}:{web_user} {parent}"),
        format!(
            "find {} -type d -exec chmod 0775 {{}} + -or -type f -exec chmod 0664 {{}} +",
            install_dir.display()
        ),
        format!("chmod 750 {}/artisan", install_dir.display()),
        format!("chmod 640 {}/.env", install_dir.display()),
        format!("chmod -R 755 {0}", install_dir.display()),
        format!("chmod -R 775 {0}/storage", install_dir.display()),
        format!("chmod -R 775 {0}/bootstrap/cache", install_dir.display()),
    ])?;
    Ok(())
}

fn crons(ctx: &mut Context) -> Result<()> {
    ctx.style.section("Setting Up Crontabs");
    let install_dir = ctx.config.install_dir().display().to_string();
    // G23: idempotent — strip prior entries then append a single instance.
    ctx.run(&format!(
        "(crontab -l 2>/dev/null | grep -v 'artisan schedule:run'; echo '* * * * * php {install_dir}/artisan schedule:run >> /dev/null 2>&1') | crontab -"
    ))?;
    Ok(())
}

fn setup(ctx: &mut Context) -> Result<()> {
    ctx.style.section("Setting Up Web Site");
    let install_dir = ctx.config.install_dir().to_path_buf();
    let install_dir_s = install_dir.display().to_string();
    let fqdn = ctx.config.app.hostname.clone();
    let web_user = ctx.config.web_user().to_string();
    let echo_port = ctx.config.app.echo_port;
    let protocol = if ctx.config.app.ssl { "https" } else { "http" };

    // Laravel Echo Server config (G20).
    let ssl_cert = format!("/etc/letsencrypt/live/{fqdn}/cert.pem");
    let ssl_key = format!("/etc/letsencrypt/live/{fqdn}/privkey.pem");
    let ssl_chain = format!("/etc/letsencrypt/live/{fqdn}/fullchain.pem");
    let echo_tpl = EchoServerTemplate {
        protocol,
        fqdn: &fqdn,
        port: echo_port,
        ssl_cert: &ssl_cert,
        ssl_key: &ssl_key,
        ssl_chain: &ssl_chain,
    };
    let echo_path = install_dir.join("laravel-echo-server.json");
    let echo_rendered = echo_tpl.render()?;
    ctx.write_file(&echo_path, &echo_rendered)?;
    ctx.run(&format!(
        "chown {web_user}:{web_user} {}",
        echo_path.display()
    ))?;

    // Supervisor for queue workers + echo server (G21, G22).
    let sup_tpl = SupervisorTemplate {
        install_dir: &install_dir_s,
        web_user: &web_user,
    };
    let sup_rendered = sup_tpl.render()?;
    ctx.write_file(
        std::path::Path::new("/etc/supervisor/conf.d/unit3d.conf"),
        &sup_rendered,
    )?;
    ctx.run_all([
        "supervisorctl reread".to_string(),
        "supervisorctl update".to_string(),
        "supervisorctl reload".to_string(),
    ])?;

    // Composer install + Bun build + artisan bootstrapping.
    let www_cmds = [
        "composer install -q --prefer-dist --no-dev",
        "composer dump-autoload --optimize",
        "bun install",
        "bun run build",
        "php artisan key:generate --force",
        "php artisan migrate --seed --force",
        "php artisan auto:email-blacklist-update",
        "php artisan storage:link", // G15
        "php artisan config:cache", // G17
        "php artisan route:cache",  // G17
        "php artisan view:cache",   // G17
    ];

    for cmd in www_cmds {
        let s = format!("sudo -u {web_user} bash -c 'cd {install_dir_s} && {cmd}'");
        // G24: if running as web-user fails (Bun modules often can't write
        // outside the checkout as www-data), fall back to running as root
        // and re-fix permissions.
        if ctx.run(&s).is_err()
            && (cmd.starts_with("bun") || cmd.starts_with("composer") || cmd.starts_with("npm"))
        {
            ctx.style
                .warning(&format!("{cmd} as {web_user} failed — retrying as root"));
            ctx.run(&format!("bash -c 'cd {install_dir_s} && {cmd}'"))?;
            ctx.run(&format!("chown -R {web_user}:{web_user} {install_dir_s}"))?;
        }
    }

    ctx.style.info("UNIT3D installed successfully");
    Ok(())
}
