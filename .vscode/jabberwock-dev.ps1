$ErrorActionPreference = "Stop"
$vs = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\18\BuildTools"
$devcmd = Join-Path $vs "Common7\Tools\VsDevCmd.bat"

if (!(Test-Path $devcmd)) { throw "MSVC初期化スクリプトがありません: $devcmd" }

$envFile = Join-Path $env:TEMP "jabberwock-msvc-$PID.txt"
$cmdFile = Join-Path $env:TEMP "jabberwock-msvc-$PID.cmd"

@(
    "@echo off"
    "call `"$devcmd`" -arch=amd64 -host_arch=amd64 >nul"
    'if errorlevel 1 exit /b 1'
    "set > `"$envFile`""
) | Set-Content -LiteralPath $cmdFile -Encoding ASCII

try {
    & $env:ComSpec /d /c "`"$cmdFile`""
    if ($LASTEXITCODE -ne 0 -or !(Test-Path $envFile)) {
        throw "MSVC環境の初期化に失敗しました"
    }

    Get-Content $envFile | ForEach-Object {
        if ($_ -match '^([^=]+)=(.*)$') {
            Set-Item "Env:$($matches[1])" $matches[2]
        }
    }
}
finally {
    Remove-Item $envFile, $cmdFile -Force -ErrorAction SilentlyContinue
}

if (!(Get-Command cl -ErrorAction SilentlyContinue)) {
    throw "cl.exe が見つかりません"
}

Set-Location (Split-Path -Parent $PSScriptRoot)
Write-Host "Jabberwock Dev Environment: READY" -ForegroundColor Green
