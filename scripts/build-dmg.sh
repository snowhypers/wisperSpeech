#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Error: Building a macOS .dmg requires macOS (Darwin)."
  exit 1
fi

npm install
npm run build:dmg

mkdir -p dist-dmg
cp -f src-tauri/target/release/bundle/dmg/*.dmg dist-dmg/
echo "DMG ready in dist-dmg/:"
ls -lh dist-dmg/*.dmg
