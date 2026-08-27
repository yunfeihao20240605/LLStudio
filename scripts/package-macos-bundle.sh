#!/usr/bin/env bash
set -euo pipefail

app_bundle=${1:?usage: package-macos-bundle.sh APP_BUNDLE ARCH}
expected_arch=${2:?usage: package-macos-bundle.sh APP_BUNDLE ARCH}
executable="$app_bundle/Contents/MacOS/els-app"
frameworks="$app_bundle/Contents/Frameworks"

[[ -x "$executable" ]] || {
  echo "macOS executable not found: $executable" >&2
  exit 1
}

brew_prefix=$(brew --prefix)
mpv_prefix=${MPV_PREFIX:-$(brew --prefix mpv)}
ffmpeg_prefix=${FFMPEG_PREFIX:-$(brew --prefix ffmpeg)}
mkdir -p "$frameworks"
install_name_tool -add_rpath "@loader_path/../Frameworks" "$executable" 2>/dev/null || true

processed=$'\n'
is_system_dependency() {
  case "$1" in
    /System/*|/usr/lib/*|/usr/lib/swift/*|/System/Library/Frameworks/*)
      return 0
      ;;
    @rpath/libSystem.*|@rpath/libc++.*|@rpath/libobjc.*|@rpath/AppKit.*|@rpath/QuartzCore.*)
      return 0
      ;;
  esac
  return 1
}

resolve_dependency() {
  local owner=$1
  local dependency=$2
  local candidate
  local name

  if [[ "$dependency" == /* && -f "$dependency" ]]; then
    printf '%s\n' "$dependency"
    return 0
  fi

  name=$(basename "$dependency")
  if [[ "$dependency" == @loader_path/* ]]; then
    candidate="$(dirname "$owner")/${dependency#@loader_path/}"
    [[ -f "$candidate" ]] && { printf '%s\n' "$candidate"; return 0; }
  elif [[ "$dependency" == @executable_path/* ]]; then
    candidate="$app_bundle/Contents/MacOS/${dependency#@executable_path/}"
    [[ -f "$candidate" ]] && { printf '%s\n' "$candidate"; return 0; }
  elif [[ "$dependency" == @rpath/* ]]; then
    candidate="$frameworks/${dependency#@rpath/}"
    [[ -f "$candidate" ]] && { printf '%s\n' "$candidate"; return 0; }
  fi

  for candidate in \
    "$frameworks/$name" \
    "$mpv_prefix/lib/$name" \
    "$ffmpeg_prefix/lib/$name"; do
    [[ -f "$candidate" ]] && { printf '%s\n' "$candidate"; return 0; }
  done

  candidate=$(find "$brew_prefix" -type f -name "$name" -print -quit 2>/dev/null || true)
  [[ -n "$candidate" ]] && printf '%s\n' "$candidate"
}

copy_dependency() {
  local owner=$1
  local dependency=$2
  local source
  local name
  local destination
  local nested

  is_system_dependency "$dependency" && return 0
  source=$(resolve_dependency "$owner" "$dependency" || true)
  if [[ -z "$source" ]]; then
    echo "Unable to resolve non-system macOS dependency $dependency (from $owner)" >&2
    exit 1
  fi

  name=$(basename "$source")
  destination="$frameworks/$name"

  # macdeployqt already owns Qt frameworks; only normalize dependencies that
  # originate outside the application bundle.
  if [[ "$source" == "$app_bundle/"* ]]; then
    return 0
  fi

  install_name_tool -change "$dependency" "@rpath/$name" "$owner" 2>/dev/null || true
  if [[ ! -f "$destination" ]]; then
    cp -L "$source" "$destination"
    chmod u+w "$destination"
    install_name_tool -id "@rpath/$name" "$destination" 2>/dev/null || true
  fi

  case "$processed" in
    *$'\n'"$name"$'\n'*) return 0 ;;
  esac
  processed+="$name"$'\n'

  while IFS= read -r nested; do
    [[ -n "$nested" ]] && copy_dependency "$destination" "$nested"
  done < <(otool -L "$destination" | tail -n +2 | sed -E 's/^[[:space:]]*([^ ]+).*/\1/')
}

while IFS= read -r dependency; do
  [[ -n "$dependency" ]] && copy_dependency "$executable" "$dependency"
done < <(otool -L "$executable" | tail -n +2 | sed -E 's/^[[:space:]]*([^ ]+).*/\1/')

for tool in "$app_bundle/Contents/MacOS/ffmpeg" "$app_bundle/Contents/MacOS/ffprobe"; do
  [[ -x "$tool" ]] || continue
  install_name_tool -add_rpath "@loader_path/../Frameworks" "$tool" 2>/dev/null || true
  while IFS= read -r dependency; do
    [[ -n "$dependency" ]] && copy_dependency "$tool" "$dependency"
  done < <(otool -L "$tool" | tail -n +2 | sed -E 's/^[[:space:]]*([^ ]+).*/\1/')
done

# libmpv can load FFmpeg components dynamically, so include every FFmpeg
# dylib rather than relying solely on libmpv's current link-time closure.
for source in \
  "$ffmpeg_prefix"/lib/libav*.dylib* \
  "$ffmpeg_prefix"/lib/libsw*.dylib* \
  "$ffmpeg_prefix"/lib/libpostproc*.dylib*; do
  [[ -f "$source" ]] || continue
  copy_dependency "$executable" "$source"
done

native_files=()
while IFS= read -r -d '' file; do
  if file -b "$file" | grep -q 'Mach-O'; then
    native_files+=("$file")
  fi
done < <(find "$app_bundle/Contents" -type f -print0)

for file in "${native_files[@]}"; do
  archs=$(lipo -archs "$file")
  if ! tr ' ' '\n' <<<"$archs" | grep -Fxq "$expected_arch"; then
    echo "Architecture mismatch: $file has [$archs], expected $expected_arch" >&2
    exit 1
  fi

  while IFS= read -r dependency; do
    [[ -n "$dependency" ]] || continue
    if [[ "$dependency" == /* ]] && ! is_system_dependency "$dependency"; then
      echo "External macOS dependency remains: $dependency (from $file)" >&2
      exit 1
    fi
  done < <(otool -L "$file" | tail -n +2 | sed -E 's/^[[:space:]]*([^ ]+).*/\1/')
done

compgen -G "$frameworks/libmpv*.dylib" >/dev/null || {
  echo "libmpv was not bundled in $frameworks" >&2
  exit 1
}
for tool in ffmpeg ffprobe; do
  [[ -x "$app_bundle/Contents/MacOS/$tool" ]] || {
    echo "$tool was not bundled in the macOS app" >&2
    exit 1
  }
done

echo "Validated macOS bundle: $app_bundle ($expected_arch)"
