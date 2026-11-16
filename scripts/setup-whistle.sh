#!/usr/bin/env bash
set -euo pipefail

OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}-${ARCH}" in
  Darwin-arm64) PLATFORM="macos-arm64" ;;
  Linux-x86_64) PLATFORM="linux-x86_64" ;;
  Linux-aarch64) PLATFORM="linux-arm64" ;;
  *) echo "Unsupported platform: ${OS}-${ARCH}"; exit 1 ;;
esac

TARGET_DIR="${HOME}/.local/share/whistleflow/whistle"
mkdir -p "${TARGET_DIR}"

echo "Downloading Cactus Whistle (16.9 MB)..."
curl -L --progress-bar "https://huggingface.co/Cactus-Compute/whistle/resolve/main/whistle.cact" -o "${TARGET_DIR}/whistle.cact"

echo "Downloading Cactus engine (${PLATFORM})..."
curl -L --progress-bar "https://huggingface.co/Cactus-Compute/cactus/resolve/main/releases/4.0.0/${PLATFORM}/cactus" -o "${TARGET_DIR}/cactus"
chmod +x "${TARGET_DIR}/cactus"

echo "Done! Saved to ${TARGET_DIR}"
