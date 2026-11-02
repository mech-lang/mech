$ErrorActionPreference = "Stop"

$needle = "integrity_invalid_host_input_aborts_staged_receiver_before_commit"

$output = @(
  cargo test -p mech-runtime --lib -- --list 2>&1
)

if ($LASTEXITCODE -ne 0) {
  $output | ForEach-Object { Write-Host $_ }
  throw "Failed to enumerate mech-runtime tests."
}

$pattern = [regex]::Escape($needle) + ": test$"
$matches = @($output | Select-String -Pattern $pattern)

if ($matches.Count -ne 1) {
  $output | ForEach-Object { Write-Host $_ }
  throw "Expected exactly one '$needle' test; found $($matches.Count)."
}

$qualifiedName =
  ($matches[0].Line -replace ": test$", "").Trim()

Write-Host "Running exactly one test: $qualifiedName"

cargo test -p mech-runtime --lib $qualifiedName -- --exact --nocapture
exit $LASTEXITCODE
