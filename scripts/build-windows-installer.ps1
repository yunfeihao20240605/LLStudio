param(
    [string]$FfmpegDir = $(if ($env:FFMPEG_DIR) { $env:FFMPEG_DIR } else { "C:\ffmpeg" }),
    [string]$MpvDevDir = $(if ($env:MPV_DEV_DIR) { $env:MPV_DEV_DIR } else { "C:\mpv-dev" }),
    [string]$MpvRuntimeDir = $(if ($env:MPV_RUNTIME_DIR) { $env:MPV_RUNTIME_DIR } else { "C:\mpv-runtime" })
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
Set-Location $projectRoot

function Require-Command([string]$Name) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name was not found in PATH"
    }
}

function Download-Asset([string]$Url, [string]$Output) {
    Write-Host "Downloading $Url"
    Invoke-WebRequest -Uri $Url -OutFile $Output
}

Require-Command "cargo"
Require-Command "7z"
Require-Command "windeployqt"

$target = "x86_64-pc-windows-gnu"
$env:FFMPEG_DIR = $FfmpegDir
$env:MPV_PREFIX = $MpvDevDir

if (-not (Test-Path (Join-Path $FfmpegDir "bin\ffmpeg.exe"))) {
    $ffmpegRelease = Invoke-RestMethod "https://api.github.com/repos/BtbN/FFmpeg-Builds/releases/latest"
    $ffmpegAsset = $ffmpegRelease.assets |
        Where-Object { $_.name -match "^ffmpeg-n\d+.*win64-gpl-shared.*\.zip$" } |
        Select-Object -First 1
    if (-not $ffmpegAsset) {
        $ffmpegAsset = $ffmpegRelease.assets |
            Where-Object { $_.name -match "^ffmpeg-master-latest-win64-(gpl|lgpl)-shared\.zip$" } |
            Select-Object -First 1
    }
    if (-not $ffmpegAsset) {
        throw "No x64 shared FFmpeg asset found"
    }

    $ffmpegZip = Join-Path $env:TEMP "llstudio-ffmpeg.zip"
    $ffmpegUnpack = Join-Path $env:TEMP "llstudio-ffmpeg"
    Remove-Item $ffmpegUnpack -Recurse -Force -ErrorAction SilentlyContinue
    Download-Asset $ffmpegAsset.browser_download_url $ffmpegZip
    Expand-Archive $ffmpegZip -DestinationPath $ffmpegUnpack -Force
    $ffmpegExe = Get-ChildItem $ffmpegUnpack -Recurse -Filter "ffmpeg.exe" | Select-Object -First 1
    if (-not $ffmpegExe) {
        throw "Downloaded FFmpeg archive has no ffmpeg.exe"
    }
    New-Item -ItemType Directory -Force $FfmpegDir | Out-Null
    Copy-Item "$($ffmpegExe.Directory.Parent.FullName)\*" $FfmpegDir -Recurse -Force
}

if (-not (Get-ChildItem $MpvDevDir -Recurse -Filter "libmpv.dll.a" -ErrorAction SilentlyContinue)) {
    $mpvRelease = Invoke-RestMethod "https://api.github.com/repos/shinchiro/mpv-winbuild-cmake/releases/latest"
    $mpvDevAsset = $mpvRelease.assets |
        Where-Object { $_.name -match "^mpv-dev-x86_64-.*\.7z$" -and $_.name -notmatch "-v3-" } |
        Select-Object -First 1
    $mpvRuntimeAsset = $mpvRelease.assets |
        Where-Object { $_.name -match "^mpv-x86_64-.*\.7z$" -and $_.name -notmatch "-v3-" } |
        Select-Object -First 1
    if (-not $mpvDevAsset -or -not $mpvRuntimeAsset) {
        throw "No x64 mpv development/runtime assets found"
    }

    $mpvDevArchive = Join-Path $env:TEMP "llstudio-mpv-dev.7z"
    $mpvRuntimeArchive = Join-Path $env:TEMP "llstudio-mpv-runtime.7z"
    Remove-Item $MpvDevDir, $MpvRuntimeDir -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force $MpvDevDir, $MpvRuntimeDir | Out-Null
    Download-Asset $mpvDevAsset.browser_download_url $mpvDevArchive
    Download-Asset $mpvRuntimeAsset.browser_download_url $mpvRuntimeArchive
    & 7z x $mpvDevArchive "-o$MpvDevDir" -y | Out-Null
    & 7z x $mpvRuntimeArchive "-o$MpvRuntimeDir" -y | Out-Null
}

$mpvLib = Get-ChildItem $MpvDevDir -Recurse -Filter "libmpv.dll.a" | Select-Object -First 1
if (-not $mpvLib) {
    throw "mpv development archive has no libmpv.dll.a"
}
$env:MPV_PREFIX = if ($mpvLib.Directory.Name -eq "lib") {
    $mpvLib.Directory.Parent.FullName
} else {
    $mpvLib.Directory.FullName
}
$env:PATH = "$FfmpegDir\bin;$env:PATH"

$qtBin = Get-Command "windeployqt" | Select-Object -ExpandProperty Source
$qtPrefix = Split-Path -Parent (Split-Path -Parent $qtBin)
if (-not $env:Qt6_DIR) {
    $env:Qt6_DIR = Join-Path $qtPrefix "lib\cmake\Qt6"
}
$env:CMAKE_PREFIX_PATH = $qtPrefix

rustup target add $target
$dist = Join-Path $projectRoot "dist"
Remove-Item $dist -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $dist | Out-Null

cargo clean --release --target $target
cargo build --release -p els-app --target $target
Copy-Item "target\$target\release\els-app.exe" $dist
Copy-Item (Get-ChildItem (Join-Path $FfmpegDir "bin") -Filter "*.dll") $dist -Force
Copy-Item (Join-Path $FfmpegDir "bin\ffmpeg.exe"), (Join-Path $FfmpegDir "bin\ffprobe.exe") $dist -Force

foreach ($runtimeDir in @($MpvRuntimeDir, $env:MPV_PREFIX)) {
    Get-ChildItem $runtimeDir -Recurse -Filter "*.dll" -File -ErrorAction SilentlyContinue |
        Copy-Item -Destination $dist -Force
}

& $qtBin --qmldir qml --compiler-runtime --no-translations (Join-Path $dist "els-app.exe")
& $qtBin --version | Out-Null

$nsis = Get-Command "makensis.exe" -ErrorAction SilentlyContinue
if (-not $nsis) {
    $nsisCandidates = @(
        (Join-Path ${env:ProgramFiles(x86)} "NSIS\makensis.exe"),
        (Join-Path $env:ProgramFiles "NSIS\makensis.exe")
    )
    $nsisPath = $nsisCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if ($nsisPath) {
        $nsis = @{ Source = $nsisPath }
    }
}
if (-not $nsis) {
    throw "makensis.exe was not found; install NSIS or add it to PATH"
}

& (Join-Path $projectRoot "scripts\validate-windows-package.ps1") -Dist $dist
& $nsis.Source (Join-Path $projectRoot "installer.nsi")
Write-Host "Built: $(Join-Path $projectRoot 'LLStudio-Setup.exe')"
