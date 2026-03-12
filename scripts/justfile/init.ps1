Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Usage {
    @"
usage:
  init.ps1 --component <components...>
  init.ps1 --install <crates...>

examples:
  ./scripts/justfile/init.ps1 --component rust-analyzer clippy rustfmt
  ./scripts/justfile/init.ps1 --install prek cargo-nextest
"@
}

function Ensure-Command {
    param([Parameter(Mandatory)][string]$Name)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "missing required command: $Name"
    }
}

function Ensure-RustupComponents {
    param([Parameter(Mandatory)][string[]]$Components)

    $installed = & rustup component list --installed 2>$null
    $missing = @()

    foreach ($component in $Components) {
        if (-not ($installed | Select-String -Pattern ("^" + [regex]::Escape($component) + "-") -Quiet)) {
            $missing += $component
        }
    }

    if ($missing.Count -gt 0) {
        & rustup component add @missing
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
    }
}

function Ensure-CargoTool {
    param([Parameter(Mandatory)][string]$Crate)

    $binary = $Crate
    if (Get-Command $binary -ErrorAction SilentlyContinue) {
        return
    }

    if (Get-Command cargo-binstall -ErrorAction SilentlyContinue) {
        & cargo binstall $Crate 2>$null
        if ($LASTEXITCODE -ne 0) {
            & cargo install --locked $Crate
            if ($LASTEXITCODE -ne 0) {
                exit $LASTEXITCODE
            }
        }
        return
    }

    & cargo install --locked $Crate
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

if ($args.Count -eq 0) {
    Usage
    exit 2
}

$i = 0
while ($i -lt $args.Count) {
    $arg = $args[$i]
    switch ($arg) {
        { $_ -in @("-h", "--help") } {
            Usage
            exit 0
        }
        "--component" {
            Ensure-Command rustup
            $i++
            $components = @()
            while ($i -lt $args.Count -and -not $args[$i].StartsWith("--")) {
                $components += $args[$i]
                $i++
            }
            if ($components.Count -eq 0) {
                throw "--component requires at least 1 value"
            }
            Ensure-RustupComponents -Components $components
        }
        "--install" {
            Ensure-Command cargo
            $i++
            $crates = @()
            while ($i -lt $args.Count -and -not $args[$i].StartsWith("--")) {
                $crates += $args[$i]
                $i++
            }
            if ($crates.Count -eq 0) {
                throw "--install requires at least 1 value"
            }
            foreach ($crate in $crates) {
                Ensure-CargoTool -Crate $crate
            }
        }
        default {
            throw "unknown argument: $arg"
        }
    }
}
