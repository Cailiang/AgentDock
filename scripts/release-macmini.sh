#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REMOTE_HOST="${MACMINI_HOST:-192.168.14.2}"
REMOTE_USER="${MACMINI_USER:-duagent}"
REMOTE_APP="${MACMINI_APP_PATH:-/Applications/AgentDock.app}"
SSH_TARGET="${REMOTE_USER}@${REMOTE_HOST}"
SSH_OPTIONS=(-o BatchMode=yes -o ConnectTimeout=10 -o StrictHostKeyChecking=accept-new)
SKIP_BUILD=0

usage() {
  printf 'Usage: %s [--skip-build]\n' "$(basename "$0")"
  printf '\nBuilds AgentDock, creates a local installer, deploys it to %s, and restarts the remote app.\n' "$SSH_TARGET"
  printf 'Override the destination with MACMINI_HOST, MACMINI_USER, or MACMINI_APP_PATH.\n'
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

for command_name in node npm rustup hdiutil shasum lipo scp ssh; do
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

DMG_PATH="$ROOT_DIR/src-tauri/target/universal-apple-darwin/release/bundle/dmg/AgentDock_${VERSION}_universal.dmg"
APP_PATH="$ROOT_DIR/src-tauri/target/universal-apple-darwin/release/bundle/macos/AgentDock.app"
LOCAL_DMG_PATH="$ROOT_DIR/release-artifacts/$(basename "$DMG_PATH")"

if [[ "$SKIP_BUILD" -eq 1 ]]; then
  "$ROOT_DIR/scripts/build-local-release.sh" --skip-build
else
  "$ROOT_DIR/scripts/build-local-release.sh"
fi

[[ -f "$DMG_PATH" && -d "$APP_PATH" ]] || {
  printf 'Release bundle not found. Run without --skip-build first.\n' >&2
  exit 1
}

LOCAL_VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP_PATH/Contents/Info.plist")"
[[ "$LOCAL_VERSION" == "$VERSION" ]] || {
  printf 'Bundle version %s does not match package version %s.\n' "$LOCAL_VERSION" "$VERSION" >&2
  exit 1
}
lipo "$APP_PATH/Contents/MacOS/agentdock" -verify_arch arm64 x86_64
hdiutil verify "$DMG_PATH" >/dev/null
DMG_SHA256="$(shasum -a 256 "$DMG_PATH" | awk '{print $1}')"
REMOTE_DIR="Library/Caches/com.shuzilm.agentdock/releases/${VERSION}-$(date +%Y%m%d%H%M%S)"
REMOTE_DMG="${REMOTE_DIR}/$(basename "$DMG_PATH")"

printf 'Uploading AgentDock %s to %s...\n' "$VERSION" "$SSH_TARGET"
ssh "${SSH_OPTIONS[@]}" "$SSH_TARGET" "mkdir -p \"\$HOME/$REMOTE_DIR\""
scp "${SSH_OPTIONS[@]}" "$DMG_PATH" "$SSH_TARGET:$REMOTE_DMG"

printf 'Installing and restarting AgentDock on %s...\n' "$SSH_TARGET"
ssh "${SSH_OPTIONS[@]}" "$SSH_TARGET" /bin/bash -s -- \
  "$VERSION" "$REMOTE_DMG" "$DMG_SHA256" "$REMOTE_APP" <<'REMOTE_SCRIPT'
set -euo pipefail

VERSION="$1"
DMG_PATH="$HOME/$2"
EXPECTED_SHA256="$3"
APP_PATH="$4"
WORK_DIR="$(dirname "$DMG_PATH")"
MOUNT_POINT="$WORK_DIR/mount"
STAGE_PATH="${APP_PATH}.update.$$"
BACKUP_PATH="${APP_PATH}.backup.$$"
MOUNTED=0
REPLACED=0

cleanup() {
  status=$?
  trap - EXIT HUP INT TERM
  if [[ "$MOUNTED" -eq 1 ]]; then
    /usr/bin/hdiutil detach "$MOUNT_POINT" -quiet || true
  fi
  /bin/rm -rf "$MOUNT_POINT" "$STAGE_PATH" "$WORK_DIR"
  if [[ "$status" -ne 0 && "$REPLACED" -eq 1 && -d "$BACKUP_PATH" ]]; then
    /bin/rm -rf "$APP_PATH"
    /bin/mv "$BACKUP_PATH" "$APP_PATH" || true
    /usr/bin/open "$APP_PATH" || true
  fi
  exit "$status"
}
trap cleanup EXIT HUP INT TERM

actual_sha256="$(/usr/bin/shasum -a 256 "$DMG_PATH" | /usr/bin/awk '{print $1}')"
[[ "$actual_sha256" == "$EXPECTED_SHA256" ]] || {
  printf 'Uploaded DMG checksum mismatch.\n' >&2
  exit 1
}

/bin/mkdir -p "$MOUNT_POINT"
/usr/bin/hdiutil attach "$DMG_PATH" -nobrowse -readonly -mountpoint "$MOUNT_POINT" -quiet
MOUNTED=1
SOURCE_APP="$(/usr/bin/find "$MOUNT_POINT" -maxdepth 2 -type d -name AgentDock.app -print -quit)"
[[ -n "$SOURCE_APP" ]] || {
  printf 'AgentDock.app was not found in the DMG.\n' >&2
  exit 1
}

/usr/bin/ditto "$SOURCE_APP" "$STAGE_PATH"
staged_version="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$STAGE_PATH/Contents/Info.plist")"
[[ "$staged_version" == "$VERSION" ]] || {
  printf 'Staged version %s does not match expected version %s.\n' "$staged_version" "$VERSION" >&2
  exit 1
}
/usr/bin/lipo "$STAGE_PATH/Contents/MacOS/agentdock" -verify_arch arm64 x86_64
/usr/bin/xattr -dr com.apple.quarantine "$STAGE_PATH" 2>/dev/null || true

/usr/bin/osascript -e 'tell application id "com.shuzilm.agentdock" to quit' >/dev/null 2>&1 || true
for _ in {1..50}; do
  /usr/bin/pgrep -f "$APP_PATH/Contents/MacOS/agentdock" >/dev/null 2>&1 || break
  /bin/sleep 0.2
done
if /usr/bin/pgrep -f "$APP_PATH/Contents/MacOS/agentdock" >/dev/null 2>&1; then
  /usr/bin/pkill -TERM -f "$APP_PATH/Contents/MacOS/agentdock" || true
  /bin/sleep 1
fi
if /usr/bin/pgrep -f "$APP_PATH/Contents/MacOS/agentdock" >/dev/null 2>&1; then
  /usr/bin/pkill -KILL -f "$APP_PATH/Contents/MacOS/agentdock" || true
  /bin/sleep 0.5
fi

if [[ -d "$APP_PATH" ]]; then
  /bin/mv "$APP_PATH" "$BACKUP_PATH"
  REPLACED=1
fi
/bin/mv "$STAGE_PATH" "$APP_PATH"

/usr/bin/open "$APP_PATH"
for _ in {1..50}; do
  if /usr/bin/pgrep -f "$APP_PATH/Contents/MacOS/agentdock" >/dev/null 2>&1; then
    installed_version="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP_PATH/Contents/Info.plist")"
    [[ "$installed_version" == "$VERSION" ]]
    /bin/rm -rf "$BACKUP_PATH"
    REPLACED=0
    printf 'AgentDock %s is running from %s\n' "$installed_version" "$APP_PATH"
    exit 0
  fi
  /bin/sleep 0.2
done

printf 'AgentDock did not restart after installation.\n' >&2
exit 1
REMOTE_SCRIPT

printf 'Release complete: AgentDock %s is running on %s.\n' "$VERSION" "$SSH_TARGET"
printf 'Local installer: %s\n' "$LOCAL_DMG_PATH"
