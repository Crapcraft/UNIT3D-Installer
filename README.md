<h1 align="center">UNIT3D Community Edition Installer</h1>

<p align="center">
    🎉<b>A Big Thanks To All Our <a href="https://github.com/HDInnovations/UNIT3D-Community-Edition/graphs/contributors">Contributors</a> and <a href="https://github.com/sponsors/HDVinnie">Sponsors</a></b>🎉
</p>

<p align="center"><b>NOTE: This only works for a fresh server with nothing on it but a new OS install!</b></p>

## This Repository

Installer for the [UNIT3D-Community-Edition](https://github.com/HDInnovations/UNIT3D-Community-Edition), rewritten in Rust as a single static binary. It replaces the legacy PHP/PHAR installer with a self-contained `curl | sh` bootstrap.

**Officially Supported OS's**
- Ubuntu 24.04 LTS (Noble Numbat)
- Ubuntu 22.04 LTS (Jammy Jellyfish)
- Ubuntu 20.04 LTS (Focal Fossa)

**Unstable WIP**
- Ubuntu 26.04 LTS

## Quick Install

Run on a fresh server with a valid A record (and CNAME for `www`) pointing at its IP:

```
curl -sSL https://raw.githubusercontent.com/InfinityHD-Net/UNIT3D-Installer/master/install.sh | sudo bash
```

Or clone and run the bootstrap manually:

```
sudo apt -y install git
git clone https://github.com/InfinityHD-Net/UNIT3D-Installer.git installer
cd installer
sudo ./install.sh
```

## Configuration

All options are declared in [`unit3d-installer.example.toml`](unit3d-installer.example.toml). Provide a config file with `--config`; any omitted field falls back to the baked-in defaults.

```
sudo ./install.sh --config /path/to/unit3d-installer.toml
```

Preview the full plan without touching the system:

```
sudo ./install.sh --dry-run --non-interactive --config unit3d-installer.example.toml
```

### Highlights

- Ubuntu LTS only; requires root (`sudo`).
- Database: MySQL, MariaDB, or PostgreSQL (pinned to the UNIT3D tag).
- PHP 8.5 (Ondrej PPA), Node 20, Bun, `laravel-echo-server`.
- Redis over unix sockets with RAM-bounded LRU eviction.
- Nginx with security headers, gzip, static-asset caching, and the `/socket.io` chat proxy on the configured echo port.
- Meilisearch (systemd service) with `scout` indexing.
- Let's Encrypt SSL via certbot.
- Queue worker under supervisor with `queue:work redis --sleep=3 --tries=3 --max-time=3600`.
- Idempotent `crontab` merge for `artisan schedule:run`.
- Credentials written to `/root/unit3d-credentials.txt` at the end.

## Building From Source

```
cargo build --release
```

The release profile uses thin LTO and symbol stripping; see `Cargo.toml`.

## Testing

```
cargo test
cargo clippy --all-targets -- -D warnings
```

## Suggestions and/or Bug Reporting

We encourage the use of [GitHub Issues](https://github.com/InfinityHD-Net/UNIT3D-Installer/issues/new)!