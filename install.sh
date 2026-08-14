#!/usr/bin/env bash
# UNIT3D-Installer bootstrap (Rust edition).
#
# Downloads a single static `unit3d-installer` binary from the latest
# GitHub Release and runs it. Replaces the legacy `install.sh` +
# `ubuntu.sh` + `box.json`/PHAR chain.

set -euo pipefail

REPO="InfinityHD-Net/UNIT3D-Installer"

if [[ $EUID -ne 0 ]]; then
    echo "ERROR: Please run as root (sudo ./install.sh)" >&2
    exit 1
fi

# Ensure we have curl + tar.
if ! command -v curl >/dev/null 2>&1; then
    apt-get -y update
    apt-get -y install -y ca-certificates curl tar
fi

ARCH="$(uname -m)"
case "$ARCH" in
    x86_64)  ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *) echo "ERROR: Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

URL="https://github.com/${REPO}/releases/latest/download/unit3d-installer-${ARCH}.tar.gz"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "Downloading $URL..."
curl -fsSL "$URL" -o "$TMP/unit3d-installer.tar.gz"
tar -xzf "$TMP/unit3d-installer.tar.gz" -C "$TMP"
install -m 0755 "$TMP/unit3d-installer" /usr/local/bin/unit3d-installer

echo "Starting UNIT3D installer..."
exec /usr/local/bin/unit3d-installer install "$@"