#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GITHUB_REPO="${AGENTDOCK_GITHUB_REPO:-Cailiang/AgentDock}"
WORKFLOW_FILE="desktop-build.yml"
REMOTE_HOST="${MACMINI_HOST:-192.168.14.2}"
REMOTE_USER="${MACMINI_USER:-duagent}"
REMOTE_APP="${MACMINI_APP_PATH:-/Applications/AgentDock.app}"
SSH_TARGET="${REMOTE_USER}@${REMOTE_HOST}"
SSH_OPTIONS=(-o BatchMode=yes -o ConnectTimeout=10 -o StrictHostKeyChecking=accept-new)

usage() {
  printf 'Usage: %s\n' "$(basename "$0")"
  printf '\nPublishes the committed main branch as a GitHub prerelease after verifying the Mac mini deployment.\n'
  printf 'Run npm run release:macmini and commit the verified changes before invoking this script.\n'
  printf 'A complete bilingual release-notes/v<version>.md file is required.\n'
}

if [[ $# -gt 0 ]]; then
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
fi

for command_name in cargo gh git jq node npm shasum ssh; do
  command -v "$command_name" >/dev/null 2>&1 || {
    printf 'Required command not found: %s\n' "$command_name" >&2
    exit 1
  }
done

cd "$ROOT_DIR"
[[ "$(git branch --show-current)" == "main" ]] || {
  printf 'Online releases must run from main.\n' >&2
  exit 1
}
[[ -z "$(git status --porcelain)" ]] || {
  printf 'The worktree is not clean. Commit the Mac mini-verified changes first.\n' >&2
  exit 1
}
gh auth status >/dev/null

VERSION="$(node -p 'require("./package.json").version')"
TAG="v${VERSION}"
RELEASE_NOTES="$ROOT_DIR/release-notes/${TAG}.md"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]] || {
  printf 'Invalid package version: %s\n' "$VERSION" >&2
  exit 1
}
"$ROOT_DIR/scripts/validate-release-notes.sh" "$RELEASE_NOTES" "$VERSION"
node - <<'NODE'
const fs = require("fs");
const packageVersion = require("./package.json").version;
const versions = [
  require("./package-lock.json").version,
  require("./src-tauri/tauri.conf.json").version,
  fs.readFileSync("src-tauri/Cargo.toml", "utf8").match(/^version = "([^"]+)"/m)?.[1]
];
if (versions.some((version) => version !== packageVersion)) {
  console.error(`Version mismatch: ${[packageVersion, ...versions].join(" / ")}`);
  process.exit(1);
}
NODE

REMOTE_VERSION="$(ssh "${SSH_OPTIONS[@]}" "$SSH_TARGET" "/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' '$REMOTE_APP/Contents/Info.plist'")"
REMOTE_PID="$(ssh "${SSH_OPTIONS[@]}" "$SSH_TARGET" "pgrep -f '$REMOTE_APP/Contents/MacOS/agentdock' | head -1" || true)"
LOCAL_APP="$ROOT_DIR/src-tauri/target/universal-apple-darwin/release/bundle/macos/AgentDock.app"
[[ -f "$LOCAL_APP/Contents/MacOS/agentdock" ]] || {
  printf 'The verified local bundle is missing. Run npm run release:macmini first.\n' >&2
  exit 1
}
LOCAL_SHA256="$(shasum -a 256 "$LOCAL_APP/Contents/MacOS/agentdock" | cut -d ' ' -f 1)"
REMOTE_SHA256="$(ssh "${SSH_OPTIONS[@]}" "$SSH_TARGET" "/usr/bin/shasum -a 256 '$REMOTE_APP/Contents/MacOS/agentdock' | /usr/bin/cut -d ' ' -f 1")"
[[ "$REMOTE_VERSION" == "$VERSION" && -n "$REMOTE_PID" && "$REMOTE_SHA256" == "$LOCAL_SHA256" ]] || {
  printf 'Mac mini is not running AgentDock %s. Run npm run release:macmini first.\n' "$VERSION" >&2
  printf 'Remote version: %s; process: %s; bundle match: %s\n' "${REMOTE_VERSION:-missing}" "${REMOTE_PID:-missing}" "$([[ "$REMOTE_SHA256" == "$LOCAL_SHA256" ]] && printf yes || printf no)" >&2
  exit 1
}

printf 'Mac mini verified: AgentDock %s (PID %s).\n' "$REMOTE_VERSION" "$REMOTE_PID"
printf 'Running release checks...\n'
npm run build:ui
cargo test --manifest-path src-tauri/Cargo.toml

git fetch origin main --tags
git merge-base --is-ancestor origin/main HEAD || {
  printf 'origin/main contains commits that are not in local main. Pull them before releasing.\n' >&2
  exit 1
}
git push origin main

verify_release() {
  local release_json expected_asset expected_checksum release_url
  expected_asset="AgentDock_${VERSION}_universal.dmg"
  expected_checksum="${expected_asset}.sha256"
  release_json="$(gh api "repos/${GITHUB_REPO}/releases/tags/${TAG}")"
  printf '%s\n' "$release_json" | jq -e --arg tag "$TAG" --arg expected "$expected_asset" --arg checksum "$expected_checksum" '
    .tag_name == $tag and
    .draft == false and
    .prerelease == true and
    (.body | contains("## 中文")) and
    (.body | contains("### 新增功能")) and
    (.body | contains("### Bug 修复")) and
    (.body | contains("## English")) and
    (.body | contains("### New Features")) and
    (.body | contains("### Bug Fixes")) and
    (.assets | length) >= 6 and
    any(.assets[]; .name == $expected) and
    any(.assets[]; .name == $checksum) and
    all(.assets[]; .size > 0 and (.digest | test("^sha256:[0-9a-f]{64}$")))
  ' >/dev/null
  release_url="$(printf '%s\n' "$release_json" | jq -r '.html_url')"
  printf 'GitHub prerelease verified: %s\n' "$release_url"
  printf '%s\n' "$release_json" | jq -r '.assets[] | "  \(.name)  \(.digest)"'
}

if gh release view "$TAG" --repo "$GITHUB_REPO" >/dev/null 2>&1; then
  verify_release
  exit 0
fi

if git show-ref --verify --quiet "refs/tags/$TAG"; then
  [[ "$(git rev-list -n 1 "$TAG")" == "$(git rev-parse HEAD)" ]] || {
    printf 'Local tag %s does not point to HEAD.\n' "$TAG" >&2
    exit 1
  }
else
  git tag -a "$TAG" -m "AgentDock ${VERSION}"
fi

REMOTE_TAG_COMMIT="$(git ls-remote --tags origin "refs/tags/${TAG}^{}" | cut -f 1)"
if [[ -n "$REMOTE_TAG_COMMIT" ]]; then
  [[ "$REMOTE_TAG_COMMIT" == "$(git rev-parse HEAD)" ]] || {
    printf 'Remote tag %s already points to another commit.\n' "$TAG" >&2
    exit 1
  }
else
  git push origin "$TAG"
fi

COMMIT="$(git rev-list -n 1 "$TAG")"
RUN_ID=""
for _ in {1..24}; do
  RUN_ID="$(gh run list --workflow "$WORKFLOW_FILE" --limit 30 --json databaseId,headBranch,headSha,event | jq -r --arg tag "$TAG" --arg commit "$COMMIT" '.[] | select(.headBranch == $tag and .headSha == $commit and .event == "push") | .databaseId' | head -1)"
  [[ -n "$RUN_ID" ]] && break
  sleep 5
done
[[ -n "$RUN_ID" ]] || {
  printf 'GitHub Actions did not create a tag build for %s.\n' "$TAG" >&2
  exit 1
}

printf 'Waiting for GitHub Actions run %s...\n' "$RUN_ID"
gh run watch "$RUN_ID" --exit-status --interval 20
verify_release
