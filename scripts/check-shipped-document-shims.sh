#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

fail() {
  echo "shipped document shim check failed: $*" >&2
  exit 1
}

require_file() {
  [[ -f "$1" ]] || fail "missing $1"
}

require_literal() {
  local file="$1"
  local literal="$2"
  grep -Fq -- "$literal" "$file" || fail "$file is missing required contract: $literal"
}

for file in include/index.html include/blog.html include/docs.html include/document.js; do
  require_file "$file"
done

command -v grep >/dev/null 2>&1 || fail "grep is required to scan shipped document shims"

for selector in \
  'id="header"' \
  'id="logo"' \
  'id="nav"' \
  'id="github"' \
  'mech-root' \
  'mech-toc' \
  'id="left-pane"' \
  'id="breadcrumb"' \
  'id="resizer"' \
  'id="toggle-repl"' \
  'console-pane' \
  'console-tabs' \
  'console-tab' \
  'console-panel' \
  'id="mech-document-output"' \
  'id="mech-document-errors"'; do
  require_literal include/index.html "$selector"
done

for selector in \
  'site-header' \
  'header-inner' \
  'contentShell' \
  'content-column' \
  'articleIntro' \
  'articleLayout' \
  'main-content' \
  'article-backmatter' \
  'post-pagination' \
  'console-pane'; do
  require_literal include/blog.html "$selector"
done

for selector in \
  'site-header' \
  'contentShell' \
  'docs-layout' \
  'docs-header' \
  'version-badge' \
  'articleIntro' \
  'article-backmatter' \
  'post-pagination' \
  'console-pane'; do
  require_literal include/docs.html "$selector"
done

for file in include/index.html include/blog.html include/docs.html; do
  for slot in \
    '{{DOCUMENT_SCRIPT}}' \
    '{{DOCUMENT_SOURCES}}' \
    '{{WASM_MODULE_URL}}' \
    '{{REPL}}' \
    '{{TITLE}}' \
    '{{CODE}}'; do
    require_literal "$file" "$slot"
  done
done

if grep -Ein -- \
  'WasmMech|CURRENT_MECH|attach_repl|/pkg/mech_wasm\.js|under construction' \
  include/index.html include/blog.html include/docs.html; then
  fail "a shipped document shim references a removed browser mechanism"
fi

if grep -Ein -- '</script' include/document.js; then
  fail "include/document.js cannot be safely embedded inside a script element"
fi

echo "shipped document shim contracts are present"
