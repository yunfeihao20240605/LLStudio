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

ffmpeg_formula="${FFMPEG_FORMULA:-ffmpeg@8.1.2}"
ffmpeg_prefix="${FFMPEG_PREFIX:-$(brew --prefix "$ffmpeg_formula" 2>/dev/null || true)}"
mpv_prefix="${MPV_PREFIX:-$(brew --prefix mpv 2>/dev/null || true)}"

[[ -n "$ffmpeg_prefix" && -d "$ffmpeg_prefix" ]] || {
  die "FFmpeg 8.1.2 is required. Install it first or set FFMPEG_PREFIX."
}
[[ -n "$mpv_prefix" && -d "$mpv_prefix" ]] || {
  die "Homebrew mpv is required. Install it first or set MPV_PREFIX."
}
[[ -x "$ffmpeg_prefix/bin/ffmpeg" ]] || {
  die "FFmpeg executable is missing from $ffmpeg_prefix"
}
[[ -f "$ffmpeg_prefix/lib/pkgconfig/libavutil.pc" ]] || {
  die "FFmpeg pkg-config files are missing from $ffmpeg_prefix"
}
[[ -f "$mpv_prefix/lib/pkgconfig/mpv.pc" ]] || {
  die "mpv pkg-config file is missing from $mpv_prefix"
}

export FFMPEG_PREFIX="$ffmpeg_prefix"
export MPV_PREFIX="$mpv_prefix"
export PATH="$ffmpeg_prefix/bin:$mpv_prefix/bin:$PATH"
export PKG_CONFIG_PATH="$ffmpeg_prefix/lib/pkgconfig:$mpv_prefix/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
export PKG_CONFIG_ALLOW_CROSS=1
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-12.0}"
export CXXFLAGS="-I$ffmpeg_prefix/include ${CXXFLAGS:-}"

if [[ -x target/debug/els-app ]]; then
  if ! otool -L target/debug/els-app | grep -q "$ffmpeg_prefix/lib/libavutil"; then
    cargo clean
  fi
fi

cargo build -p els-app
exec target/debug/els-app
