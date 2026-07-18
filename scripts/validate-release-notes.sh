#!/bin/bash
set -euo pipefail

NOTES_FILE="${1:-}"
VERSION="${2:-}"

usage() {
  printf 'Usage: %s <release-notes-file> <version>\n' "$(basename "$0")"
}

if [[ -z "$NOTES_FILE" || -z "$VERSION" || $# -ne 2 ]]; then
  usage >&2
  exit 2
fi

[[ -f "$NOTES_FILE" ]] || {
  printf 'Release notes not found: %s\n' "$NOTES_FILE" >&2
  exit 1
}

require_line() {
  local line="$1"
  grep -Fqx "$line" "$NOTES_FILE" || {
    printf 'Release notes are missing required heading: %s\n' "$line" >&2
    exit 1
  }
}

require_section_item() {
  local heading="$1"
  awk -v heading="$heading" '
    $0 == heading { active = 1; next }
    active && ($0 ~ /^### / || $0 ~ /^## /) { exit found ? 0 : 1 }
    active && $0 ~ /^- / { found = 1 }
    END { if (active) exit found ? 0 : 1 }
  ' "$NOTES_FILE" || {
    printf 'Release notes section has no list items: %s\n' "$heading" >&2
    exit 1
  }
}

require_line "# AgentDock ${VERSION}"
require_line "## 中文"
require_line "### 新增功能"
require_line "### Bug 修复"
require_line "## English"
require_line "### New Features"
require_line "### Bug Fixes"

for heading in "### 新增功能" "### Bug 修复" "### New Features" "### Bug Fixes"; do
  require_section_item "$heading"
done

if grep -Eq 'TODO|TBD|待补充|待完善|在此填写|PLACEHOLDER' "$NOTES_FILE"; then
  printf 'Release notes still contain template placeholders: %s\n' "$NOTES_FILE" >&2
  exit 1
fi

chinese_line="$(grep -Fn '## 中文' "$NOTES_FILE" | head -1 | cut -d: -f1)"
english_line="$(grep -Fn '## English' "$NOTES_FILE" | head -1 | cut -d: -f1)"
[[ "$chinese_line" -lt "$english_line" ]] || {
  printf 'The Chinese release notes must appear before the English release notes.\n' >&2
  exit 1
}

printf 'Bilingual release notes verified: %s\n' "$NOTES_FILE"
