param(
    [Parameter(Mandatory = $true)]
    [string]$Dist
)

$ErrorActionPreference = "Stop"
$objdumpCommand = Get-Command objdump.exe -ErrorAction SilentlyContinue
if (-not $objdumpCommand) {
    $objdumpCommand = Get-Command llvm-objdump.exe -ErrorAction Stop
}
$objdump = $objdumpCommand.Source
$nativeFiles = Get-ChildItem -Path $Dist -Recurse -File |
    Where-Object { $_.Extension -in @(".exe", ".dll") }

if (-not (Test-Path (Join-Path $Dist "els-app.exe"))) {
    throw "Installer payload does not contain els-app.exe"
}
foreach ($required in @("Qt6Core.dll", "Qt6Gui.dll", "Qt6Qml.dll", "Qt6Quick.dll")) {
    if (-not (Test-Path (Join-Path $Dist $required))) {
        throw "Qt runtime is missing $required"
    }
}
if (-not (Get-ChildItem (Join-Path $Dist "qml") -ErrorAction SilentlyContinue)) {
    throw "Qt QML runtime directory is missing"
}
if (-not (Get-ChildItem $Dist -Filter "*mpv*.dll" -File)) {
    throw "libmpv runtime DLL is missing"
}
foreach ($tool in @("ffmpeg.exe", "ffprobe.exe")) {
    if (-not (Test-Path (Join-Path $Dist $tool))) {
        throw "FFmpeg runtime tool is missing: $tool"
    }
}

function Test-SystemDll([string]$Name) {
    if ($Name -match "^(api-ms-win-|ext-ms-)") {
        return $true
    }
    return Test-Path (Join-Path $env:SystemRoot "System32\$Name")
}

foreach ($file in $nativeFiles) {
    $headers = (& $objdump -f $file.FullName 2>&1 | Out-String)
    if ($LASTEXITCODE -ne 0 -or $headers -notmatch "i386:x86-64|pei-x86-64") {
        throw "Architecture mismatch or invalid PE file: $($file.FullName)"
    }

    $dependents = (& $objdump -p $file.FullName 2>&1 | Out-String)
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to inspect dependencies: $($file.FullName)"
    }
    $names = [regex]::Matches($dependents, "(?im)^\s*DLL Name:\s*([A-Za-z0-9_.-]+\.dll)\s*$") |
        ForEach-Object { $_.Groups[1].Value } |
        Sort-Object -Unique
    foreach ($name in $names) {
        if (-not (Test-Path (Join-Path $Dist $name)) -and -not (Test-SystemDll $name)) {
            throw "Unresolved Windows dependency $name (from $($file.Name))"
        }
    }
}

Write-Host "Validated Windows payload: $Dist (x64)"
