Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Usage {
    @"
usage:
  web.ps1 install

environment:
  AGENTSTOW_WEB_INSTALL=missing|always|never  default: missing
  AGENTSTOW_WEB_PM=auto|bun|npm               default: auto
"@
}

function Get-RepoRoot {
    $scriptDir = Split-Path -Parent $PSCommandPath
    return (Resolve-Path (Join-Path $scriptDir "../..")).Path
}

function Invoke-BunInstall {
    param([Parameter(Mandatory)][string]$WebDir)

    if (-not (Get-Command bun -ErrorAction SilentlyContinue)) {
        return $false
    }

    Push-Location $WebDir
    try {
        & bun install --frozen-lockfile
        return ($LASTEXITCODE -eq 0)
    } finally {
        Pop-Location
    }
}

function Invoke-NpmInstall {
    param([Parameter(Mandatory)][string]$WebDir)

    if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
        throw "missing required command: npm"
    }

    Push-Location $WebDir
    try {
        if (Test-Path "package-lock.json") {
            & npm ci
        } else {
            & npm install
        }

        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
    } finally {
        Pop-Location
    }
}

function Install-WebDeps {
    $repoRoot = Get-RepoRoot
    $webDir = Join-Path $repoRoot "web"
    $mode = if ($env:AGENTSTOW_WEB_INSTALL) { $env:AGENTSTOW_WEB_INSTALL } else { "missing" }
    $pm = if ($env:AGENTSTOW_WEB_PM) { $env:AGENTSTOW_WEB_PM } else { "auto" }

    switch ($mode) {
        "never" {
            Write-Host "[agentstow:web-install] skip install (AGENTSTOW_WEB_INSTALL=never)"
            return
        }
        "missing" {
            if (Test-Path (Join-Path $webDir "node_modules")) {
                Write-Host "[agentstow:web-install] reuse existing web/node_modules; skip network install"
                return
            }
        }
        "always" {
        }
        default {
            throw "unsupported AGENTSTOW_WEB_INSTALL=$mode"
        }
    }

    switch ($pm) {
        "bun" {
            if (-not (Invoke-BunInstall -WebDir $webDir)) {
                throw "bun install failed"
            }
        }
        "npm" {
            Invoke-NpmInstall -WebDir $webDir
        }
        "auto" {
            if (-not (Invoke-BunInstall -WebDir $webDir)) {
                Write-Warning "[agentstow:web-install] bun install failed; fallback to npm"
                Invoke-NpmInstall -WebDir $webDir
            }
        }
        default {
            throw "unsupported AGENTSTOW_WEB_PM=$pm"
        }
    }
}

if ($args.Count -eq 0) {
    Usage
    exit 2
}

switch ($args[0]) {
    "install" {
        Install-WebDeps
        exit 0
    }
    { $_ -in @("-h", "--help", "help") } {
        Usage
        exit 0
    }
    default {
        throw "unsupported command: $($args[0])"
    }
}
