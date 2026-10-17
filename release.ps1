$ErrorActionPreference = 'Stop'

$exe = Join-Path $PSScriptRoot 'target\release\autoclicker.exe'

cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$len = (Get-Item $exe).Length
Write-Host ("built {0} -> {1} bytes ({2:N1} KB)" -f $exe, $len, ($len / 1KB))
