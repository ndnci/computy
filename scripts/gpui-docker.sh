#!/usr/bin/env bash
# Run a command in the GPUI toolchain container with the checkout mounted.
# Example: scripts/gpui-docker.sh cargo build --release -p computy-desktop
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
if ! docker image inspect computy-gpui:local >/dev/null 2>&1 || [[ "${COMPUTY_REBUILD_IMAGE:-}" == 1 ]]; then
  docker build --target gpui -t computy-gpui:local "$root"
fi
docker volume create computy-cargo >/dev/null
docker run --rm -v computy-cargo:/cargo computy-gpui:local sh -c "mkdir -p /cargo && cp -rn /usr/local/cargo/bin /cargo/ 2>/dev/null; chown -R $(id -u):$(id -g) /cargo" >/dev/null
tty=()
[[ -t 0 ]] && tty=(-t)
exec docker run --rm -i "${tty[@]}" -u "$(id -u):$(id -g)" \
  -e HOME=/tmp -e PATH=/cargo/bin:/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
  -e RUSTUP_HOME=/usr/local/rustup -e CARGO_TARGET_DIR=/workspace/desktop/target -e CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-8}" \
  -v computy-cargo:/cargo -v "$root:/workspace" -w /workspace computy-gpui:local "$@"
