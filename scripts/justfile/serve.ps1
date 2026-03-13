$argsList = @($args)
$addr = if ($env:AGENTSTOW_ADDR) { $env:AGENTSTOW_ADDR } else { '127.0.0.1:8787' }

if ($argsList.Length -gt 0 -and -not $argsList[0].StartsWith('-')) {
    $addr = $argsList[0]
    if ($argsList.Length -gt 1) {
        $argsList = $argsList[1..($argsList.Length - 1)]
    } else {
        $argsList = @()
    }
}

& cargo run -p agentstow-cli -- @argsList serve --addr $addr
exit $LASTEXITCODE
