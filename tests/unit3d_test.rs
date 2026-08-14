//! UNIT3D, Meilisearch, and PHP step integration tests. Drive the steps
//! through the mocked executor in dry-run mode and assert the emitted
//! commands and file writes.

mod common;

use common::test_context_dry;
use unit3d_installer::steps::Step;
use unit3d_installer::steps::{meilisearch::MeilisearchSetupStep, unit3d::Unit3dSetupStep};

fn unit3d_context() -> (unit3d_installer::steps::Context, common::MockExec) {
    let (mut ctx, _exec) = test_context_dry();
    ctx.config.app.hostname = "tracker.example.com".to_string();
    ctx.config.app.owner = "UNIT3D".to_string();
    ctx.config.app.owner_email = "admin@tracker.example.com".to_string();
    ctx.config.app.db = "unit3d".to_string();
    ctx.config.app.dbuser = "unit3d".to_string();
    ctx.config.app.dbpass = "secretpass".to_string();
    ctx.config.app.dbrootpass = "rootpw".to_string();
    ctx.config.app.meilisearch_key = "0123456789abcdef0123456789abcdef".to_string();
    ctx.config.unit3d.tag = "v9.2.0".to_string();
    ctx.config.unit3d.repository =
        "https://github.com/HDInnovations/UNIT3D-Community-Edition.git".to_string();
    (ctx, _exec)
}

#[test]
fn unit3d_clones_tag_pinned_repo() {
    let (mut ctx, exec) = unit3d_context();
    Unit3dSetupStep.handle(&mut ctx).unwrap();

    // G14: tag-pinned clone with safe.directory.
    assert!(exec.any("git config --global --add safe.directory /var/www/html"));
    assert!(exec.any("git clone -b v9.2.0 https://github.com/HDInnovations/UNIT3D-Community-Edition.git /var/www/html"));
}

#[test]
fn unit3d_installs_dependencies_and_bootstraps() {
    let (mut ctx, exec) = unit3d_context();
    Unit3dSetupStep.handle(&mut ctx).unwrap();

    // Composer + Bun.
    assert!(exec.any("composer install -q --prefer-dist --no-dev"));
    assert!(exec.any("composer dump-autoload --optimize"));
    assert!(exec.any("bun install"));
    assert!(exec.any("bun run build"));
    // Artisan bootstrapping (G15/G17).
    assert!(exec.any("php artisan key:generate"));
    assert!(exec.any("php artisan migrate --seed --force"));
    assert!(exec.any("php artisan storage:link"));
    assert!(exec.any("php artisan config:cache"));
    assert!(exec.any("php artisan route:cache"));
    assert!(exec.any("php artisan view:cache"));
}

#[test]
fn unit3d_sets_permissions_and_cron() {
    let (mut ctx, exec) = unit3d_context();
    Unit3dSetupStep.handle(&mut ctx).unwrap();

    assert!(exec.any("chown -R www-data:www-data"));
    assert!(exec.any("chmod 750 /var/www/html/artisan"));
    assert!(exec.any("chmod 640 /var/www/html/.env"));
    // G23: idempotent cron merge.
    assert!(exec.any("crontab -l 2>/dev/null | grep -v 'artisan schedule:run'"));
    assert!(exec.any("artisan schedule:run >> /dev/null 2>&1"));
}

#[test]
fn unit3d_installs_supervisor_and_echo_server() {
    let (mut ctx, exec) = unit3d_context();
    Unit3dSetupStep.handle(&mut ctx).unwrap();

    assert!(exec.any("supervisorctl reread"));
    assert!(exec.any("supervisorctl update"));
    assert!(exec.any("supervisorctl reload"));
    // Echo server config chowned to web user.
    assert!(exec.any("chown www-data:www-data /var/www/html/laravel-echo-server.json"));
}

#[test]
fn meilisearch_configures_service_and_scout() {
    let (mut ctx, exec) = unit3d_context();
    MeilisearchSetupStep.handle(&mut ctx).unwrap();

    assert!(exec.any("systemctl daemon-reload"));
    assert!(exec.any("systemctl enable meilisearch"));
    assert!(exec.any("systemctl start meilisearch"));
    // Scout index settings + import.
    assert!(exec.any("php artisan scout:sync-index-settings"));
    assert!(exec.any("php artisan scout:import"));
}

#[test]
fn php_step_patches_ini_and_www_conf() {
    use unit3d_installer::steps::php::PhpSetupStep;

    let (mut ctx, exec) = test_context_dry();
    ctx.config.app.hostname = "tracker.example.com".to_string();
    PhpSetupStep.handle(&mut ctx).unwrap();

    // The step should not fail on a box without PHP, and emits nothing
    // when no php.ini files are found (glob empty). It must return Ok.
    // This is a smoke test — actual sed commands are covered by the
    // patch_ini / patch_www unit tests in the step module.
    let _ = exec;
}
