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

reject_literal() {
  local file="$1"
  local literal="$2"
  if grep -Fq -- "$literal" "$file"; then
    fail "$file contains app-specific chrome: $literal"
  fi
}

for file in include/index.html include/blog.html include/docs.html include/document.js src/wasm/src/repl.rs; do
  require_file "$file"
done

command -v grep >/dev/null 2>&1 || fail "grep is required to scan shipped document shims"

for selector in \
  'mech-root' \
  'contentShell' \
  'content-column' \
  'articleIntro' \
  'articleLayout' \
  'main-content' \
  'article-backmatter' \
  'id="resizer"' \
  'console-pane' \
  'console-tabs' \
  'console-tab' \
  'console-panel' \
  'id="mech-document-output"' \
  'id="mech-document-errors"'; do
  require_literal include/index.html "$selector"
done

for selector in \
  'contentShell' \
  'content-column' \
  'articleIntro' \
  'articleLayout' \
  'main-content' \
  'article-backmatter' \
  'console-pane'; do
  require_literal include/blog.html "$selector"
done

for selector in \
  'contentShell' \
  'docs-layout' \
  'docs-header' \
  'version-badge' \
  'articleIntro' \
  'article-backmatter' \
  'console-pane'; do
  require_literal include/docs.html "$selector"
done

for file in include/index.html include/blog.html include/docs.html; do
  reject_literal "$file" '<header class="site-header">'
  reject_literal "$file" '<footer class="footer">'
  reject_literal "$file" 'data-mech-console-toggle'
  reject_literal "$file" 'class="breadcrumbs"'
  reject_literal "$file" 'class="mika-separator"'
  reject_literal "$file" 'class="post-pagination"'
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

for contract in \
  'input.dataset.mechInteractiveEvaluation = "resident"' \
  'state.repl.invoke(source)' \
  'Mech resident REPL input'; do
  require_literal include/document.js "$contract"
done

require_literal src/wasm/src/repl.rs \
  'console_output_context: "console://repl/output".to_string()'

if grep -Fq -- 'state.document.evaluate' include/document.js || \
  grep -Fq -- 'supportsInteractiveEvaluation' include/document.js; then
  fail "the shipped document controller retains a developer-evaluation path"
fi

echo "shipped document shim contracts are present"
