$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $repoRoot

function Assert-Command {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name,
    [string]$InstallHint = ""
  )

  if (Get-Command $Name -ErrorAction SilentlyContinue) {
    return
  }

  $message = "$Name is required."
  if ($InstallHint) {
    $message += " Install it with:`n$InstallHint"
  }
  throw $message
}

try {
  Assert-Command "cargo"
  Assert-Command "rustup"
  Assert-Command "wasm-pack" "cargo install wasm-pack --locked"

  $nativeArtifact = "target/release/mech.exe"

  # Do not let a stale native executable make a failed build look complete.
  Remove-Item $nativeArtifact -Force -ErrorAction SilentlyContinue

  python scripts/build-wasm.py --profile browser-compute
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to build the Mech browser/WASM product."
  }

  cargo build --locked --release --features compute_backends_native
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to build the native Mech resident-executor product."
  }

  $artifacts = @(
    $nativeArtifact
    "src/wasm/pkg/mech_wasm.js"
    "src/wasm/pkg/mech_wasm_bg.wasm"
  )

  foreach ($artifact in $artifacts) {
    if (!(Test-Path $artifact -PathType Leaf)) {
      throw "Build completed without expected artifact: $artifact"
    }
  }

  Write-Host "Built the complete Mech resident-executor product:"
  foreach ($artifact in $artifacts) {
    Write-Host "  $artifact"
  }
} finally {
  Pop-Location
}
