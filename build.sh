#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(cd "$(dirname "$0")" && pwd)
ARTIFACTS_DIR="${REPO_ROOT}/artifacts/YT-Panel-NEW"
FRONTEND_DIST_DIR="${ARTIFACTS_DIR}/frontend-dist"
BACKEND_TARGET_DIR="${ARTIFACTS_DIR}/backend-target"
RELEASE_DIR="${ARTIFACTS_DIR}/release"
RUST_TARGET="${RUST_TARGET:-x86_64-unknown-linux-musl}"
DOCKER_IMAGE_NAME="${DOCKER_IMAGE_NAME:-yt-panel:local}"
CT_TEMPLATE_NAME="${CT_TEMPLATE_NAME:-yt-panel-alpine-ct-template.tar.zst}"
FRONTEND="false"
BACKEND="false"
PACKAGE="false"
IMAGE="false"
TEMPLATE="false"

ensure_dirs() {
  mkdir -p "$FRONTEND_DIST_DIR" "$BACKEND_TARGET_DIR" "$RELEASE_DIR"
}

build_frontend() {
  ensure_dirs
  cd "$REPO_ROOT"
  YT_PANEL_DIST_OUT_DIR="$FRONTEND_DIST_DIR" npm run build
}

build_backend() {
  ensure_dirs
  cd "$REPO_ROOT/backend"
  cargo build --release --target "$RUST_TARGET" --target-dir "$BACKEND_TARGET_DIR"
}

ensure_docker_inputs() {
  if [[ ! -f "$REPO_ROOT/dist/index.html" ]]; then
    cd "$REPO_ROOT"
    npm run build-only
  fi

  if [[ ! -f "$REPO_ROOT/backend/target/${RUST_TARGET}/release/yt-panel-rust-backend" ]]; then
    cd "$REPO_ROOT/backend"
    cargo build --release --target "$RUST_TARGET"
  fi
}

build_image() {
  ensure_docker_inputs
  cd "$REPO_ROOT"
  docker build -t "$DOCKER_IMAGE_NAME" .
}

export_ct_template() {
  ensure_dirs

  if ! command -v docker >/dev/null 2>&1; then
    echo "docker is required to export a CT template" >&2
    exit 1
  fi

  if ! command -v zstd >/dev/null 2>&1; then
    echo "zstd is required to export a CT template" >&2
    exit 1
  fi

  local cid
  cid=$(docker create "$DOCKER_IMAGE_NAME")
  trap 'docker rm -f "$cid" >/dev/null 2>&1 || true' RETURN

  docker export "$cid" | zstd -19 -T0 -o "${RELEASE_DIR}/${CT_TEMPLATE_NAME}" -f
}

package_release() {
  ensure_dirs
  local package_root="${RELEASE_DIR}/yt-panel-linux-amd64"
  rm -rf "$package_root"
  mkdir -p "$package_root/conf" "$package_root/web"

  cp "$BACKEND_TARGET_DIR/${RUST_TARGET}/release/yt-panel-rust-backend" "$package_root/yt-panel"
  cp "$REPO_ROOT/backend/config/docker.toml" "$package_root/conf/app.toml"
  cp -R "$FRONTEND_DIST_DIR/." "$package_root/web/"

  tar -C "$RELEASE_DIR" -czf "${RELEASE_DIR}/yt-panel-linux-amd64.tar.gz" "yt-panel-linux-amd64"
}

usage() {
  cat <<'EOF'
Usage: ./build.sh [-f] [-b] [-r]
  -f  build frontend into artifacts/YT-Panel-NEW/frontend-dist
  -b  build Rust backend musl release binary into artifacts/YT-Panel-NEW/backend-target
  -r  build frontend + backend and package a release tarball under artifacts/YT-Panel-NEW/release
  -i  build the Alpine Docker image defined by DOCKER_IMAGE_NAME
  -t  build the Alpine Docker image and export a PVE-ready CT template tar.zst
EOF
}

while getopts "fbrith" opt; do
  case "$opt" in
    f) FRONTEND="true" ;;
    b) BACKEND="true" ;;
    r) FRONTEND="true"; BACKEND="true"; PACKAGE="true" ;;
    i) IMAGE="true" ;;
    t) IMAGE="true"; TEMPLATE="true" ;;
    h) usage; exit 0 ;;
    *) usage; exit 1 ;;
  esac
done

if [[ "$FRONTEND" == "false" && "$BACKEND" == "false" && "$PACKAGE" == "false" && "$IMAGE" == "false" && "$TEMPLATE" == "false" ]]; then
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

if [[ "$IMAGE" == "true" ]]; then
  build_image
fi

if [[ "$TEMPLATE" == "true" ]]; then
  export_ct_template
fi
