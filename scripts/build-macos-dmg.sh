#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"

die() {
  echo "error: $*" >&2
  exit 1
}

command -v brew >/dev/null 2>&1 || die "Homebrew is required"
command -v cargo >/dev/null 2>&1 || die "Rust/Cargo is required"

version="$(cargo pkgid -p els-app | sed 's/.*#//')"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$ ]] || die "unable to determine a valid els-app version"

arch="$(uname -m)"
case "$arch" in
  arm64)
    target="aarch64-apple-darwin"
    ffmpeg_formula="local/ffmpeg8/ffmpeg@8.1.2"
    ;;
  x86_64)
    target="x86_64-apple-darwin"
    ffmpeg_formula="local/ffmpeg8/ffmpeg@8.1.2"
    ;;
  *)
    die "unsupported macOS architecture: $arch"
    ;;
esac

brew install qt mpv pkg-config create-dmg

if ! brew tap | grep -Fxq "local/ffmpeg8"; then
  brew tap-new --no-git local/ffmpeg8
fi
ffmpeg_tap_dir="$(brew --repository local/ffmpeg8)"
ffmpeg_formula_file="$ffmpeg_tap_dir/Formula/ffmpeg@8.1.2.rb"
if [[ ! -f "$ffmpeg_formula_file" ]]; then
  mkdir -p "$(dirname "$ffmpeg_formula_file")"
  curl -fsSL \
    "https://api.github.com/repos/Homebrew/homebrew-core/contents/Formula/f/ffmpeg.rb?ref=c8d371d10663c637fe24eafcdf6bcd5cd0bec4ea" |
    python3 -c 'import base64,json,sys; print(base64.b64decode(json.load(sys.stdin)["content"]).decode(), end="")' |
    sed 's/^class Ffmpeg < Formula$/class FfmpegAT812 < Formula/' > "$ffmpeg_formula_file"
fi
brew trust --formula "$ffmpeg_formula"
ffmpeg8_prefix="$(brew --prefix ffmpeg@8.1.2)"
if [[ ! -x "$ffmpeg8_prefix/bin/ffmpeg" || ! -x "$ffmpeg8_prefix/bin/ffprobe" ]]; then
  brew reinstall --formula --build-from-source "$ffmpeg_formula_file"
fi

qt_prefix="${QT_PREFIX:-$(brew --prefix qt)}"
ffmpeg_prefix="${FFMPEG_PREFIX:-$(brew --prefix "$ffmpeg_formula")}"
mpv_prefix="${MPV_PREFIX:-$(brew --prefix mpv)}"
pkgconf_prefix="$(brew --prefix pkgconf)"
macdeployqt_bin="${MACDEPLOYQT:-$qt_prefix/bin/macdeployqt}"
[[ -x "$macdeployqt_bin" ]] || die "macdeployqt not found: $macdeployqt_bin"

export Qt6_DIR="${Qt6_DIR:-$qt_prefix/lib/cmake/Qt6}"
export CMAKE_PREFIX_PATH="$qt_prefix"
export FFMPEG_FORMULA="$ffmpeg_formula"
export FFMPEG_PREFIX="$ffmpeg_prefix"
export MPV_PREFIX="$mpv_prefix"
export PATH="$ffmpeg_prefix/bin:$mpv_prefix/bin:$pkgconf_prefix/bin:$PATH"
export PKG_CONFIG_PATH="$ffmpeg_prefix/lib/pkgconfig:$mpv_prefix/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
export PKG_CONFIG_ALLOW_CROSS=1
export "PKG_CONFIG_ALLOW_CROSS_${target//-/_}"=1
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-12.0}"
export CXXFLAGS="-I$qt_prefix/include ${CXXFLAGS:-}"

if ! pkg-config --exists libavutil mpv; then
  die "FFmpeg/mpv pkg-config files are unavailable; check PKG_CONFIG_PATH=$PKG_CONFIG_PATH"
fi
[[ -x "$ffmpeg_prefix/bin/ffmpeg" ]] || die "ffmpeg executable not found"
[[ -x "$ffmpeg_prefix/bin/ffprobe" ]] || die "ffprobe executable not found"

dist="$project_root/dist"
app_bundle="$dist/LLStudio.app"
dmg="$dist/LLStudio-macOS-$arch-$version.dmg"
rm -rf "$dist"
mkdir -p "$app_bundle/Contents/MacOS" "$app_bundle/Contents/Resources"

cargo clean --release --target "$target"
cargo build --release -p els-app --target "$target"
cp "crates/els-app/Info.plist" "$app_bundle/Contents/Info.plist"
cp "target/$target/release/els-app" "$app_bundle/Contents/MacOS/els-app"

"$macdeployqt_bin" "$app_bundle" -qmldir=qml -always-overwrite
# macdeployqt handles the linked QtSvg framework; copy the SVG image plugin
# explicitly because the SVG files are QRC resources.
mkdir -p "$app_bundle/Contents/PlugIns/imageformats"
cp "$qt_prefix/plugins/imageformats/libqsvg.dylib" \
  "$app_bundle/Contents/PlugIns/imageformats/libqsvg.dylib"
rm -rf "$app_bundle/Contents/PlugIns/sqldrivers"
cp resources/icons/macos/LLStudio.icns "$app_bundle/Contents/Resources/LLStudio.icns"
cp "$ffmpeg_prefix/bin/ffmpeg" "$app_bundle/Contents/MacOS/ffmpeg"
cp "$ffmpeg_prefix/bin/ffprobe" "$app_bundle/Contents/MacOS/ffprobe"

bash scripts/package-macos-bundle.sh "$app_bundle" "$arch"

create-dmg \
  --volname "English Learning Studio" \
  --window-pos 200 120 \
  --window-size 800 400 \
  --icon-size 100 \
  --app-drop-link 600 185 \
  "$dmg" \
  "$app_bundle"

echo "Built: $dmg"
