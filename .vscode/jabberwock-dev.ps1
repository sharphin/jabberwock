$ErrorActionPreference = "Stop"

# Workspace-local MSVC setup; only modifies this PowerShell process.
$programFilesX86 = [Environment]::GetEnvironmentVariable('ProgramFiles(x86)')
$vs = Join-Path $programFilesX86 'Microsoft Visual Studio\18\BuildTools'
$vcRoot = Join-Path $vs 'VC\Tools\MSVC'
$kits = Join-Path $programFilesX86 'Windows Kits\10'

if (!(Test-Path $vcRoot)) {
    throw "MSVC tools directory not found: $vcRoot"
}

# Prefer the newest installed MSVC toolset, independent of VsDevCmd discovery.
$toolset = Get-ChildItem -LiteralPath $vcRoot -Directory |
    Where-Object { $_.Name -match '^\d+(\.\d+){2,3}$' } |
    Sort-Object { [version]$_.Name } -Descending |
    Select-Object -First 1

if (!$toolset) {
    throw "No MSVC toolset version found under $vcRoot"
}

$vc = $toolset.FullName
$clDir = Join-Path $vc 'bin\Hostx64\x64'
$clExe = Join-Path $clDir 'cl.exe'
$vcInclude = Join-Path $vc 'include'

if (!(Test-Path $clExe)) {
    throw "MSVC compiler not found: $clExe. In Visual Studio Installer, add the MSVC x64/x86 build tools component."
}
if (!(Test-Path (Join-Path $vcInclude 'vcruntime.h'))) {
    throw "MSVC header vcruntime.h not found under $vcInclude. Repair the MSVC component in Visual Studio Installer."
}

# Find an installed Windows SDK that contains the UCRT headers.
$sdkIncludeRoot = Join-Path $kits 'Include'
$sdk = Get-ChildItem -LiteralPath $sdkIncludeRoot -Directory |
    Where-Object { Test-Path (Join-Path $_.FullName 'ucrt\math.h') } |
    Sort-Object { try { [version]$_.Name } catch { [version]'0.0' } } -Descending |
    Select-Object -First 1

if (!$sdk) {
    throw "Windows SDK UCRT math.h not found under $sdkIncludeRoot. Add/repair a Windows 10/11 SDK in Visual Studio Installer."
}

$sdkVersion = $sdk.Name
$sdkInc = $sdk.FullName
$sdkLib = Join-Path (Join-Path $kits 'Lib') $sdkVersion

# These variables apply only to this terminal process.
$env:VCToolsInstallDir = "$vc\"
$env:VCToolsVersion = $toolset.Name
$env:VSINSTALLDIR = "$vs\"
$env:WindowsSdkDir = "$kits\"
$env:WindowsSDKVersion = "$sdkVersion\"

$env:PATH = "$clDir;$env:PATH"
$includePaths = @(
    $vcInclude
    (Join-Path $sdkInc 'ucrt')
    (Join-Path $sdkInc 'shared')
    (Join-Path $sdkInc 'um')
    (Join-Path $sdkInc 'winrt')
    (Join-Path $sdkInc 'cppwinrt')
) | Where-Object { Test-Path $_ }
$env:INCLUDE = (($includePaths + @($env:INCLUDE -split ';' | Where-Object { $_ })) -join ';')

$libPaths = @(
    (Join-Path $vc 'lib\x64')
    (Join-Path $sdkLib 'ucrt\x64')
    (Join-Path $sdkLib 'um\x64')
) | Where-Object { Test-Path $_ }
$env:LIB = (($libPaths + @($env:LIB -split ';' | Where-Object { $_ })) -join ';')

if (!(Get-Command cl.exe -ErrorAction SilentlyContinue)) {
    throw 'cl.exe still cannot be resolved after setting the MSVC path.'
}
if (!(Test-Path (Join-Path $env:VCToolsInstallDir 'include\vcruntime.h'))) {
    throw 'vcruntime.h is not available in the selected MSVC toolset.'
}

# Verify that Clang can include the Windows C runtime header.
$probe = Join-Path $env:TEMP "jabberwock-header-check-$PID.c"
try {
    Set-Content -LiteralPath $probe -Encoding ASCII -Value @(
        '#include <math.h>'
        'int main(void) { return 0; }'
    )
    & clang -x c -fsyntax-only $probe
    if ($LASTEXITCODE -ne 0) {
        throw 'Clang could not compile a source file including math.h.'
    }
}
finally {
    Remove-Item -LiteralPath $probe -Force -ErrorAction SilentlyContinue
}

Set-Location (Split-Path -Parent $PSScriptRoot)
Write-Host 'Jabberwock Dev Environment: READY' -ForegroundColor Green
Write-Host "MSVC: $($toolset.Name) | Windows SDK: $sdkVersion"
