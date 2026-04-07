#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  cat <<'EOF' >&2
Usage: ./scripts/export-pve-template.sh <docker-image> <output-template.tar.zst>
EOF
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required" >&2
  exit 1
fi

if ! command -v zstd >/dev/null 2>&1; then
  echo "zstd is required" >&2
  exit 1
fi

IMAGE_NAME="$1"
OUTPUT_PATH="$2"

mkdir -p "$(dirname "$OUTPUT_PATH")"

cid=$(docker create "$IMAGE_NAME")
trap 'docker rm -f "$cid" >/dev/null 2>&1 || true' EXIT

docker export "$cid" | zstd -19 -T0 -o "$OUTPUT_PATH" -f
