#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SKIP_BUILD=0

usage() {
  printf 'Usage: %s [--skip-build]\n' "$(basename "$0")"
  printf '\nBuilds and validates a universal macOS DMG, then copies it to release-artifacts/.\n'
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-build)
      SKIP_BUILD=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'Unknown option: %s\n\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

for command_name in node npm rustup hdiutil shasum lipo; do
  command -v "$command_name" >/dev/null 2>&1 || {
    printf 'Required command not found: %s\n' "$command_name" >&2
    exit 1
  }
done

cd "$ROOT_DIR"
VERSION="$(node -p 'require("./package.json").version')"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]] || {
  printf 'Invalid package version: %s\n' "$VERSION" >&2
  exit 1
}

BUILD_DIR="$ROOT_DIR/src-tauri/target/universal-apple-darwin/release/bundle"
DMG_PATH="$BUILD_DIR/dmg/AgentDock_${VERSION}_universal.dmg"
APP_PATH="$BUILD_DIR/macos/AgentDock.app"
EXECUTABLE_PATH="$APP_PATH/Contents/MacOS/agentdock"
ARTIFACT_DIR="$ROOT_DIR/release-artifacts"
ARTIFACT_PATH="$ARTIFACT_DIR/$(basename "$DMG_PATH")"
CHECKSUM_PATH="$ARTIFACT_PATH.sha256"

if [[ "$SKIP_BUILD" -eq 0 ]]; then
  printf 'Building AgentDock %s for Intel and Apple silicon Macs...\n' "$VERSION"
  rustup target add aarch64-apple-darwin x86_64-apple-darwin
  npm run build -- --target universal-apple-darwin
fi

[[ -f "$DMG_PATH" && -d "$APP_PATH" && -f "$EXECUTABLE_PATH" ]] || {
  printf 'Release bundle not found. Run without --skip-build first.\n' >&2
  exit 1
}

BUNDLE_VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP_PATH/Contents/Info.plist")"
[[ "$BUNDLE_VERSION" == "$VERSION" ]] || {
  printf 'Bundle version %s does not match package version %s.\n' "$BUNDLE_VERSION" "$VERSION" >&2
  exit 1
}

lipo "$EXECUTABLE_PATH" -verify_arch x86_64 arm64
hdiutil verify "$DMG_PATH" >/dev/null

mkdir -p "$ARTIFACT_DIR"
cp -p "$DMG_PATH" "$ARTIFACT_PATH.tmp"
mv "$ARTIFACT_PATH.tmp" "$ARTIFACT_PATH"
(
  cd "$ARTIFACT_DIR"
  shasum -a 256 "$(basename "$ARTIFACT_PATH")" > "$(basename "$CHECKSUM_PATH")"
)

printf 'Local installer ready: %s\n' "$ARTIFACT_PATH"
printf 'Architectures: %s\n' "$(lipo -archs "$EXECUTABLE_PATH")"
printf 'SHA-256: %s\n' "$(awk '{print $1}' "$CHECKSUM_PATH")"
