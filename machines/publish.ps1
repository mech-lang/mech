param (
    [Parameter(Mandatory = $true)]
    [string]$RootDir
)

# Get only immediate subdirectories
$crateDirs = Get-ChildItem -Path $RootDir -Directory

foreach ($dir in $crateDirs) {
    $path = $dir.FullName
    $cargoToml = Join-Path $path "Cargo.toml"

    if (-Not (Test-Path $cargoToml)) {
        Write-Host "Skipping $path (no Cargo.toml)" -ForegroundColor DarkGray
        continue
    }

    Write-Host "`nProcessing $path" -ForegroundColor Cyan
    Push-Location $path

    # --- Improved Git repo detection ---
    $isGitRepo = $false
    try {
        git rev-parse --is-inside-work-tree > $null 2>&1
        if ($LASTEXITCODE -eq 0) { $isGitRepo = $true }
    } catch {
        $isGitRepo = $false
    }

    if ($isGitRepo) {
        git add -A

        git diff --cached --quiet
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Changes found. Committing..." -ForegroundColor Green
            git commit -m "bump version"
            git push
        } else {
            Write-Host "No changes to commit." -ForegroundColor Yellow
        }

        Write-Host "Publishing crate..." -ForegroundColor Magenta
        cargo publish
    } else {
        Write-Host "Skipping $path (not a Git repo)" -ForegroundColor Red
    }

    Pop-Location
}
