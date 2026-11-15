# 将 Cargo 的固定目标名复制为带项目版本的 Windows 发布程序。
$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    $metadataText = & cargo metadata --locked --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $metadata = $metadataText | ConvertFrom-Json
    $package = @($metadata.packages | Where-Object { $_.name -eq 'toolbox' })
    if ($package.Count -ne 1) { throw '无法确定 toolbox 包版本。' }
    & cargo build --locked --release --bin Toolbox_rust
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $source = Join-Path $metadata.target_directory 'release/Toolbox_rust.exe'
    $destination = Join-Path $PSScriptRoot ("dist/Toolbox_rust_v{0}.exe" -f $package[0].version)
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
    Copy-Item -LiteralPath $source -Destination $destination -Force
    Write-Host "已生成：$destination"
} finally {
    Pop-Location
}
