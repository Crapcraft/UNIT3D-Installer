//! Snapshot tests for the rendered config-file templates. These lock the
//! generated `.env`, nginx site, mysql config, php-fpm pool, supervisor,
//! meilisearch service, and echo-server JSON against golden files, so the
//! installer's output can't silently drift.

use askama::Template;

use unit3d_installer::resources::credentials::CredentialsTemplate;
use unit3d_installer::resources::echo_server::EchoServerTemplate;
use unit3d_installer::resources::env::EnvTemplate;
use unit3d_installer::resources::meilisearch_toml::MeilisearchTomlTemplate;
use unit3d_installer::resources::meilisearch_unit::MeilisearchUnitTemplate;
use unit3d_installer::resources::my_cnf::MyCnfTemplate;
use unit3d_installer::resources::nginx::NginxTemplate;
use unit3d_installer::resources::phpfpm::PhpFpmTemplate;
use unit3d_installer::resources::supervisor::SupervisorTemplate;

fn render<T: Template>(tpl: &T) -> String {
    tpl.render().unwrap()
}

#[test]
fn nginx_site_snapshot() {
    insta::assert_snapshot!(render(&NginxTemplate {
        fqdn: "tracker.example.com",
        install_dir: "/var/www/unit3d",
        echo_port: 8443,
        max_body: "256M",
    }));
}

#[test]
fn env_snapshot() {
    insta::assert_snapshot!(render(&EnvTemplate {
        protocol: "https",
        fqdn: "tracker.example.com",
        db_driver: "mariadb",
        db: "unit3d",
        dbuser: "unit3d",
        dbpass: "secret",
        socket: "/var/run/redis/redis.sock",
        owner: "admin",
        owner_email: "admin@tracker.example.com",
        owner_password: "ownerpass",
        tmdb_key: "tmdbkey",
        mail_driver: "smtp",
        mail_host: "smtp.gmail.com",
        mail_port: "587",
        mail_username: "user@example.com",
        mail_password: "mailpass",
        mail_from_name: "UNIT3D",
        meilisearch_key: "masterkey",
        redis_host: "127.0.0.1",
        redis_port: "6379",
    }));
}

#[test]
fn php_fpm_snapshot() {
    insta::assert_snapshot!(render(&PhpFpmTemplate {
        fqdn: "tracker.example.com",
        web_user: "www-data",
    }));
}

#[test]
fn supervisor_snapshot() {
    insta::assert_snapshot!(render(&SupervisorTemplate {
        install_dir: "/var/www/unit3d",
        web_user: "www-data",
    }));
}

#[test]
fn meilisearch_unit_snapshot() {
    insta::assert_snapshot!(render(&MeilisearchUnitTemplate {
        web_user: "www-data",
    }));
}

#[test]
fn my_cnf_snapshot() {
    insta::assert_snapshot!(render(&MyCnfTemplate {
        password: "rootsecret",
    }));
}

#[test]
fn echo_server_snapshot() {
    insta::assert_snapshot!(render(&EchoServerTemplate {
        protocol: "https",
        fqdn: "tracker.example.com",
        port: 8443,
        ssl_cert: "/etc/letsencrypt/live/tracker.example.com/fullchain.pem",
        ssl_key: "/etc/letsencrypt/live/tracker.example.com/privkey.pem",
        ssl_chain: "/etc/letsencrypt/live/tracker.example.com/chain.pem",
    }));
}

#[test]
fn meilisearch_toml_snapshot() {
    insta::assert_snapshot!(render(&MeilisearchTomlTemplate {
        master_key: "masterkey",
        db_path: "/var/lib/meilisearch/data.ms",
        dump_dir: "/var/lib/meilisearch/dumps",
        snapshot_dir: "/var/lib/meilisearch/snapshots",
    }));
}

#[test]
fn credentials_snapshot() {
    insta::assert_snapshot!(render(&CredentialsTemplate {
        generated: "2026-08-07",
        fqdn: "tracker.example.com",
        owner: "admin",
        owner_email: "admin@tracker.example.com",
        owner_password: "ownerpass",
        db_name: "unit3d",
        db_user: "unit3d",
        db_pass: "dbpass",
        db_root_pass: "rootpass",
        meilisearch_key: "masterkey",
        install_dir: "/var/www/unit3d",
        php_version: "8.5",
        web_user: "www-data",
    }));
}
