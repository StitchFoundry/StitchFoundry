#!/usr/bin/env bash
# Install CI apt packages without depending on the Ubuntu mirrors when a cache is warm.
# Usage: packaging/ci/apt-deps.sh CACHE_DIR PKG...
# CACHE_DIR is the directory actions/cache restores/saves (key: package list + runner image).
#   1. Everything already installed (dpkg -s)  -> nothing to do.
#   2. CACHE_DIR holds .deb files               -> dpkg -i them, no apt-get update.
#   3. Otherwise                                -> apt-get update + install (with retries),
#                                                  then copy the downloaded debs into CACHE_DIR.
set -euo pipefail
cache=${1:?cache dir}
shift
[ "$#" -gt 0 ] || { echo "no packages given" >&2; exit 2; }

if dpkg -s "$@" >/dev/null 2>&1; then
  echo "apt deps already installed"
  exit 0
fi

mkdir -p "$cache"
if compgen -G "$cache/*.deb" >/dev/null; then
  echo "installing from cache $cache"
  if sudo dpkg -i "$cache"/*.deb && dpkg -s "$@" >/dev/null 2>&1; then
    exit 0
  fi
  echo "cached debs did not satisfy the install; falling back to apt" >&2
  rm -f "$cache"/*.deb
fi

if [ -f /etc/apt/apt-mirrors.txt ] && [ "$(dpkg --print-architecture)" = amd64 ]; then
  # Ubuntu-host DNS outage 2026-11-11: try mirrors.kernel.org first, then the Azure mirror.
  printf "http://mirrors.kernel.org/ubuntu/\nhttp://azure.archive.ubuntu.com/ubuntu/\n" | sudo tee /etc/apt/apt-mirrors.txt >/dev/null
  sudo sed -i -E "s#https?://(security|archive|azure\.archive)\.ubuntu\.com/ubuntu#http://mirrors.kernel.org/ubuntu#g" /etc/apt/sources.list.d/ubuntu.sources 2>/dev/null || true
fi
for _ in 1 2 3; do sudo apt-get -o Acquire::Retries=3 update && break; sleep 20; done
sudo apt-get -o Acquire::Retries=3 install -y --no-install-recommends "$@"
# Keep what apt downloaded (only what the image lacked) for the next run.
cp -f /var/cache/apt/archives/*.deb "$cache"/ 2>/dev/null || true
