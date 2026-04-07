#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 3 || $# -gt 4 ]]; then
  cat <<'EOF' >&2
Usage: ./scripts/prepare-release-files.sh <release-dir> <binary-path> <web-dir> [config-path]
EOF
  exit 1
fi

RELEASE_DIR="$1"
BINARY_PATH="$2"
WEB_DIR="$3"
CONFIG_PATH="${4:-backend/config/docker.toml}"

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
PACKAGE_ROOT="${RELEASE_DIR}/yt-panel-linux-amd64"

rm -rf "$PACKAGE_ROOT"
mkdir -p "$PACKAGE_ROOT/conf" "$PACKAGE_ROOT/web"

cp "$BINARY_PATH" "$PACKAGE_ROOT/yt-panel"
cp "$REPO_ROOT/LICENSE" "$PACKAGE_ROOT/LICENSE"
cp "$REPO_ROOT/$CONFIG_PATH" "$PACKAGE_ROOT/conf/app.toml"
cp -R "$WEB_DIR/." "$PACKAGE_ROOT/web/"

tar -C "$RELEASE_DIR" -czf "${RELEASE_DIR}/yt-panel-linux-amd64.tar.gz" yt-panel-linux-amd64
