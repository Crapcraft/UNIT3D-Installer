//! Snapshot tests for the rendered config-file templates. These lock the
//! generated `.env`, nginx site, mysql config, php-fpm pool, supervisor,
//! meilisearch service, and echo-server JSON against golden files, so the
//! installer's output can't silently drift.

use askama::Template;

use unit3d_installer::resources::credentials::CredentialsTemplate;
use unit3d_installer::resources::echo_server::EchoServerTemplate;
use unit3d_installer::resources::env::EnvTemplate;
use unit3d_installer::resources::intro::IntroTemplate;
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

#[test]
fn intro_snapshot() {
    insta::assert_snapshot!(render(&IntroTemplate));
}

#[test]
fn env_snapshot_postgres_no_socket() {
    insta::assert_snapshot!(render(&EnvTemplate {
        protocol: "http",
        fqdn: "tracker.example.com",
        db_driver: "pgsql",
        db: "unit3d",
        dbuser: "unit3d",
        dbpass: "secret",
        socket: "",
        owner: "admin",
        owner_email: "admin@tracker.example.com",
        owner_password: "ownerpass",
        tmdb_key: "",
        mail_driver: "smtp",
        mail_host: "smtp.gmail.com",
        mail_port: "587",
        mail_username: "user@example.com",
        mail_password: "mailpass",
        mail_from_name: "UNIT3D",
        meilisearch_key: "masterkey",
        redis_host: "/var/run/redis/redis.sock",
        redis_port: "-1",
    }));
}

#[test]
fn echo_server_snapshot_http_no_ssl() {
    insta::assert_snapshot!(render(&EchoServerTemplate {
        protocol: "http",
        fqdn: "tracker.example.com",
        port: 6001,
        ssl_cert: "",
        ssl_key: "",
        ssl_chain: "",
    }));
}

#[test]
fn my_cnf_snapshot_empty_password() {
    insta::assert_snapshot!(render(&MyCnfTemplate { password: "" }));
}

#[test]
fn credentials_snapshot_http_variant() {
    insta::assert_snapshot!(render(&CredentialsTemplate {
        generated: "2026-08-07T00:00:00Z",
        fqdn: "sub.example.com",
        owner: "owner",
        owner_email: "owner@example.com",
        owner_password: "opass",
        db_name: "unit3d",
        db_user: "unit3d",
        db_pass: "dpass",
        db_root_pass: "rpass",
        meilisearch_key: "mkey",
        install_dir: "/srv/unit3d",
        php_version: "8.5",
        web_user: "ubuntu",
    }));
}

#[test]
fn echo_server_json_is_valid_json() {
    // Laravel Echo Server reads this file as strict JSON — it must parse.
    let out = render(&EchoServerTemplate {
        protocol: "https",
        fqdn: "tracker.example.com",
        port: 8443,
        ssl_cert: "/etc/letsencrypt/live/tracker.example.com/fullchain.pem",
        ssl_key: "/etc/letsencrypt/live/tracker.example.com/privkey.pem",
        ssl_chain: "/etc/letsencrypt/live/tracker.example.com/chain.pem",
    });
    let v: serde_json::Value = serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("echo-server JSON invalid: {e}\n{out}"));
    assert_eq!(v["port"], 8443);
    assert_eq!(v["authHost"], "https://tracker.example.com");
}

#[test]
fn echo_server_http_variant_is_valid_json() {
    let out = render(&EchoServerTemplate {
        protocol: "http",
        fqdn: "tracker.example.com",
        port: 6001,
        ssl_cert: "",
        ssl_key: "",
        ssl_chain: "",
    });
    let v: serde_json::Value = serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("echo-server JSON invalid: {e}\n{out}"));
    assert_eq!(v["port"], 6001);
}

#[test]
fn meilisearch_toml_is_valid_toml() {
    // Meilisearch fails to start on a malformed config — the rendered TOML
    // must round-trip through a parser.
    let out = render(&MeilisearchTomlTemplate {
        master_key: "masterkey",
        db_path: "/var/lib/meilisearch/data.ms",
        dump_dir: "/var/lib/meilisearch/dumps",
        snapshot_dir: "/var/lib/meilisearch/snapshots",
    });
    let v: toml::Value =
        toml::from_str(&out).unwrap_or_else(|e| panic!("meilisearch TOML invalid: {e}\n{out}"));
    assert_eq!(v["env"], toml::Value::String("production".into()));
    assert!(v["master_key"].as_str().is_some());
}

#[test]
fn my_cnf_is_valid_ini_style() {
    // `[client]` header plus a password= line — a comment-safe shape.
    let out = render(&MyCnfTemplate {
        password: "rootsecret",
    });
    assert!(out.starts_with("[client]"), "got: {out}");
    assert!(out.contains("password=rootsecret"), "got: {out}");
    // Empty password must still produce a valid file (no trailing garbage).
    let empty = render(&MyCnfTemplate { password: "" });
    assert!(empty.contains("password="), "got: {empty}");
}

#[test]
fn env_file_has_no_broken_lines() {
    // Every non-empty line must be `KEY=value` shaped — a stray newline in a
    // value would silently corrupt the .env Laravel parses.
    let out = render(&EnvTemplate {
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
    });
    for line in out.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        assert!(
            line.contains('=') && !line.starts_with('='),
            "env line is not KEY=value: {line}"
        );
        assert_eq!(
            line.matches('=').count(),
            1,
            "env line has extra '=': {line}"
        );
    }
}

#[test]
fn echo_port_consistency_between_echo_and_env() {
    // The nginx proxy block and laravel-echo-server.json must agree on the
    // port; VITE_ECHO_ADDRESS in .env must too.
    let echo = render(&EchoServerTemplate {
        protocol: "https",
        fqdn: "tracker.example.com",
        port: 8443,
        ssl_cert: "",
        ssl_key: "",
        ssl_chain: "",
    });
    let v: serde_json::Value = serde_json::from_str(&echo).unwrap();
    let port = v["port"].as_u64().unwrap();
    let env = render(&EnvTemplate {
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
    });
    let echo_line = env
        .lines()
        .find(|l| l.starts_with("VITE_ECHO_ADDRESS="))
        .expect("VITE_ECHO_ADDRESS must exist");
    assert!(echo_line.contains(&format!(":{port}")), "{echo_line}");
}
