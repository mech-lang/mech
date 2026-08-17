$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $repoRoot

try {
  rustup target add wasm32-unknown-unknown
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to install the wasm32-unknown-unknown Rust target."
  }

  Remove-Item "src/wasm/pkg" -Recurse -Force -ErrorAction SilentlyContinue
  wasm-pack build src/wasm `
    --target web `
    --out-dir pkg `
    --no-default-features `
    --features browser_project,browser_compute
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to build the Mech GPU browser compiler."
  }

  if (!(Test-Path "src/wasm/pkg/mech_wasm.js") -or
      !(Test-Path "src/wasm/pkg/mech_wasm_bg.wasm")) {
    throw "The Mech GPU browser package is incomplete."
  }

  $wasmGlue = Get-Content "src/wasm/pkg/mech_wasm.js" -Raw
  if (!$wasmGlue.Contains("export class WasmMixedComputeProject") -or
      !$wasmGlue.Contains("static fromSource(")) {
    throw "The Mech compute browser package does not export WasmMixedComputeProject.fromSource."
  }
} finally {
  Pop-Location
}
