#!/bin/sh
set -eu

ip link set lo up 2>/dev/null || true
ip link set eth0 up 2>/dev/null || true

if command -v udhcpc >/dev/null 2>&1; then
  udhcpc -q -n -i eth0 >/dev/null 2>&1 || true
fi

exec /usr/local/bin/container-init
