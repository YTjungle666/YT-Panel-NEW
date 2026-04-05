#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(cd "$(dirname "$0")" && pwd)
WORKSPACE_ROOT=$(cd "$REPO_ROOT/../.." && pwd)
ARTIFACTS_DIR="${WORKSPACE_ROOT}/artifacts/YT-Panel-NEW"
FRONTEND_DIST_DIR="${ARTIFACTS_DIR}/frontend-dist"
BACKEND_TARGET_DIR="${ARTIFACTS_DIR}/backend-target"
RELEASE_DIR="${ARTIFACTS_DIR}/release"
BACKEND_RELEASE_DIR="${RELEASE_DIR}/backend"
FRONTEND="false"
BACKEND="false"
PACKAGE="false"

ensure_dirs() {
  mkdir -p "$FRONTEND_DIST_DIR" "$BACKEND_TARGET_DIR" "$BACKEND_RELEASE_DIR"
}

build_frontend() {
  ensure_dirs
  cd "$REPO_ROOT"
  YT_PANEL_DIST_OUT_DIR="$FRONTEND_DIST_DIR" npm run build
}

build_backend() {
  ensure_dirs
  cd "$REPO_ROOT/backend"
  cargo build --release --target-dir "$BACKEND_TARGET_DIR"
}

package_release() {
  ensure_dirs
  local package_root="${RELEASE_DIR}/yt-panel-new-linux-amd64"
  rm -rf "$package_root"
  mkdir -p "$package_root/backend/config" "$package_root/frontend-dist"

  cp "$BACKEND_TARGET_DIR/release/yt-panel-rust-backend" "$package_root/backend/yt-panel-rust-backend"
  cp "$REPO_ROOT/backend/config/example.toml" "$package_root/backend/config/app.toml"
  cp -R "$FRONTEND_DIST_DIR/." "$package_root/frontend-dist/"

  tar -C "$RELEASE_DIR" -czf "${RELEASE_DIR}/yt-panel-new-linux-amd64.tar.gz" "yt-panel-new-linux-amd64"
}

usage() {
  cat <<'EOF'
Usage: ./build.sh [-f] [-b] [-r]
  -f  build frontend into artifacts/YT-Panel-NEW/frontend-dist
  -b  build Rust backend release binary into artifacts/YT-Panel-NEW/backend-target
  -r  build frontend + backend and package a release tarball under artifacts/YT-Panel-NEW/release
EOF
}

while getopts "fbrh" opt; do
  case "$opt" in
    f) FRONTEND="true" ;;
    b) BACKEND="true" ;;
    r) FRONTEND="true"; BACKEND="true"; PACKAGE="true" ;;
    h) usage; exit 0 ;;
    *) usage; exit 1 ;;
  esac
done

if [[ "$FRONTEND" == "false" && "$BACKEND" == "false" && "$PACKAGE" == "false" ]]; then
  usage
  exit 1
fi

if [[ "$FRONTEND" == "true" ]]; then
  build_frontend
fi

if [[ "$BACKEND" == "true" ]]; then
  build_backend
fi

if [[ "$PACKAGE" == "true" ]]; then
  package_release
fi
