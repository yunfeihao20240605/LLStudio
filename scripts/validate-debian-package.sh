#!/usr/bin/env bash
set -euo pipefail

package=${1:?usage: validate-debian-package.sh PACKAGE.deb}
binary=$(dpkg-deb -c "$package" | awk '$6 == "./usr/bin/els-app" { print $6; exit }')
[[ -n "$binary" ]] || { echo "els-app is missing from $package" >&2; exit 1; }

architecture=$(dpkg-deb -f "$package" Architecture)
[[ "$architecture" == "amd64" ]] || {
  echo "Unexpected Debian architecture: $architecture" >&2
  exit 1
}

depends=$(dpkg-deb -f "$package" Depends)
for dependency in ffmpeg libmpv; do
  grep -Fq "$dependency" <<<"$depends" || {
    echo "Debian metadata does not declare $dependency: $depends" >&2
    exit 1
  }
done

payload="$package.extract"
rm -rf "$payload"
mkdir "$payload"
dpkg-deb -x "$package" "$payload"
executable="$payload/usr/bin/els-app"

elf_header=$(readelf -h "$executable")
grep -Eq 'Class:[[:space:]]+ELF64' <<<"$elf_header" &&
  grep -Eq 'Machine:[[:space:]]+Advanced Micro Devices X86-64' <<<"$elf_header" || {
  echo "els-app is not an amd64 ELF binary" >&2
  exit 1
}
if ldd "$executable" 2>&1 | grep -q 'not found'; then
  echo "Unresolved ELF dependency in Debian payload" >&2
  ldd "$executable" >&2
  exit 1
fi
if readelf -d "$executable" | grep -Eq '/(home|Users|opt|usr/local|runner|build)/|[A-Za-z]:[/\\]'; then
  echo "Build-machine path remains in ELF dynamic metadata" >&2
  exit 1
fi

rm -rf "$payload"
echo "Validated Debian package: $package (amd64)"
