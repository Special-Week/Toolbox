[CmdletBinding()]
param(
    [switch]$Release,
    [switch]$BuildCpp,
    [string]$Mingw32Bin = $env:TOOLBOX_MINGW32_BIN,
    [string]$Mingw64Bin = $env:TOOLBOX_MINGW64_BIN
)

$ErrorActionPreference = 'Stop'

function Invoke-Check {
    param([string]$Program, [string[]]$Arguments)
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Program $($Arguments -join ' ') failed (exit $LASTEXITCODE)."
    }
}

function Invoke-CppCheck {
    param([string]$Architecture, [string]$Bin)
    $originalPath = $env:Path
    Push-Location "$PSScriptRoot/cpp_version"
    try {
        if ($Bin) { $env:Path = "$Bin;$originalPath" }
        $targets = if ($Release -or $BuildCpp) { @('test', 'all') } else { @('test') }
        Invoke-Check mingw32-make (@('-s', '-j4', "ARCH=$Architecture") + $targets)
    } finally {
        $env:Path = $originalPath
        Pop-Location
    }
}

if ($Mingw32Bin) { $Mingw32Bin = (Resolve-Path -LiteralPath $Mingw32Bin).Path }
if ($Mingw64Bin) { $Mingw64Bin = (Resolve-Path -LiteralPath $Mingw64Bin).Path }
if ($Release -and (!$Mingw32Bin -or !$Mingw64Bin)) {
    throw 'Release verification requires both -Mingw32Bin and -Mingw64Bin (or the corresponding TOOLBOX_MINGW*_BIN variables).'
}

Push-Location $PSScriptRoot
try {
    Invoke-Check cargo @('check', '--locked')
    Invoke-Check cargo @('test', '--locked')
    Invoke-Check cargo @('test', '--locked', '--manifest-path', 'web_version/core/Cargo.toml')

    Invoke-Check node @('cpp_version/scripts/sync-math-catalog.mjs', '--check')
    foreach ($name in 'tables.json', 'unified.json') {
        if ((Get-FileHash "data/gauges/$name").Hash -ne
            (Get-FileHash "cpp_version/resources/gauges/$name").Hash) {
            throw "Gauge data copies differ: $name"
        }
    }

    Push-Location web_version
    try {
        Invoke-Check npm.cmd @('ci')
        Invoke-Check npm.cmd @('run', 'build')
        Invoke-Check npm.cmd @('test')
    } finally {
        Pop-Location
    }

    if ($Release) { Invoke-Check cargo @('build', '--locked', '--release') }
    if ($Mingw64Bin) { Invoke-CppCheck x64 $Mingw64Bin }
    if ($Mingw32Bin) { Invoke-CppCheck x86 $Mingw32Bin }
    if (!$Mingw64Bin -and !$Mingw32Bin) {
        $target = & gcc -dumpmachine
        if ($LASTEXITCODE -ne 0) { throw 'Could not determine the MinGW architecture.' }
        if ($target -like 'i686-*') { Invoke-CppCheck x86 }
        elseif ($target -like 'x86_64-*') { Invoke-CppCheck x64 }
        else { throw "Expected a MinGW compiler, found $target." }
    }

    Write-Host 'Toolbox verification passed.'
} finally {
    Pop-Location
}
