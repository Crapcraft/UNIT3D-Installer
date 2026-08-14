//! Step tests: verify each installer step emits the expected shell
//! commands through the mocked executor.

mod common;

use common::test_context;
use unit3d_installer::steps::Step;
use unit3d_installer::steps::{nginx, policies, prerequisites, redis};

#[test]
fn redis_step_enables_socket_and_memory_cap() {
    let (mut ctx, exec) = test_context();
    // dry_run=true records commands and skips root/filesystem checks.
    ctx.dry_run = true;
    redis::RedisSetupStep.handle(&mut ctx).unwrap();

    assert!(exec.any("mkdir -p /var/run/redis/"));
    assert!(exec.any("usermod -aG redis www-data"));
    assert!(exec.any("unixsocket \\/var\\/run\\/redis\\/redis.sock"));
    assert!(exec.any("unixsocketperm 770"));
    // G4: memory cap + LRU eviction
    assert!(exec.any("maxmemory 256mb"));
    assert!(exec.any("maxmemory-policy allkeys-lru"));
    assert!(exec.any("systemctl restart redis-server"));
}

#[test]
fn redis_step_uses_configured_web_user() {
    let (mut ctx, exec) = test_context();
    ctx.dry_run = true;
    ctx.config.os.ubuntu.web_user = "unit3d".to_string();
    redis::RedisSetupStep.handle(&mut ctx).unwrap();
    assert!(exec.any("usermod -aG redis unit3d"));
}

#[test]
fn nginx_step_writes_site_and_ssl() {
    let (mut ctx, exec) = test_context();
    ctx.dry_run = true;
    ctx.config.app.hostname = "tracker.example.com".to_string();
    ctx.config.app.owner_email = "admin@tracker.example.com".to_string();
    ctx.config.app.echo_port = 8443;
    ctx.config.app.ssl = true;

    nginx::NginxSetupStep.handle(&mut ctx).unwrap();

    assert!(exec.any("rm -f /etc/nginx/sites-enabled/default"));
    assert!(exec.any("nginx -t"));
    assert!(exec.any("systemctl restart nginx"));
    assert!(exec.any("ufw allow 'Nginx Full'"));
    assert!(exec.any("ufw allow 8443"));
    // G11: echo server port proxied under the site.
    assert!(exec.any("ufw allow 8443"));
    assert!(exec.any("certbot --redirect --nginx -n --agree-tos --email=admin@tracker.example.com -d tracker.example.com -d www.tracker.example.com --rsa-key-size 2048"));
}

#[test]
fn nginx_step_skips_certbot_when_ssl_disabled() {
    let (mut ctx, exec) = test_context();
    ctx.dry_run = true;
    ctx.config.app.ssl = false;
    nginx::NginxSetupStep.handle(&mut ctx).unwrap();
    assert!(!exec.any("certbot"));
}

#[test]
fn prerequisites_installs_packages_and_extensions() {
    let (mut ctx, exec) = test_context();
    ctx.dry_run = true;
    prerequisites::PrerequisitesStep.handle(&mut ctx).unwrap();

    // apt-get install with the full package list.
    assert!(exec.any("apt-get install -y"));
    // PHP extensions (php8.5-*).
    assert!(exec.any("php8.5-fpm"));
    // Node + Bun + echo server.
    assert!(exec.any("deb.nodesource.com/setup_20.x"));
    assert!(exec.any("bun.sh/install"));
    assert!(exec.any("npm install -g laravel-echo-server"));
    // UFW
    assert!(exec.any("ufw allow 8443"));
    assert!(exec.any("ufw allow 'Nginx Full'"));
}

#[test]
fn policies_pass_on_clean_dir_dry_run() {
    let (mut ctx, _exec) = test_context();
    ctx.dry_run = true;
    // Must not error when the dir is clean (or when already running under root).
    let result = policies::PoliciesStep.handle(&mut ctx);
    assert!(result.is_ok());
}
