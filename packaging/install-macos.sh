#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
[ "$(uname -s)" = Darwin ] || { echo 'This installer is for macOS.' >&2; exit 1; }
base=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
arch=$(uname -m)
# BUILD.json records the host architecture this package was built for.
if [ -f "$base/BUILD.json" ]; then
 pkg_platform=$(sed -n 's/.*"platform"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$base/BUILD.json" | head -n 1)
else
 pkg_platform=unknown
fi
case "$arch:$pkg_platform" in
 arm64:macos-x64)
  echo 'Note: this is the Intel (x86_64) build; it will run on Apple Silicon through Rosetta 2.'
  echo '      For a native arm64 build, install the macos-arm64 package instead.'
  ;;
 x86_64:macos-arm64)
  echo 'This package is built for Apple Silicon (arm64) and cannot run on an Intel host.' >&2
  exit 1
  ;;
esac
for f in otp otpc libotp_rtl.a; do
 [ -f "$base/$f" ] || { echo "Missing package file: $f" >&2; exit 1; }
done
prefix=${OTP_PREFIX:-"$HOME/.local"}
dest="$prefix/lib/odin-turbo-pascal/2.0.0-dev.2"
# Per-user default. Set OTP_APP_PREFIX=/Applications to install the .app
# system-wide (requires write permission to that directory).
app_prefix=${OTP_APP_PREFIX:-"$HOME/Applications"}
mkdir -p "$dest" "$prefix/bin" "$app_prefix"
cp -R "$base/docs" "$base/examples" "$base/assets" "$base/third-party-licenses" "$dest/"
cp "$base/LICENSE" "$base/README.md" "$base/THIRD_PARTY.md" "$base/BUILD.json" "$base/libotp_rtl.a" "$dest/"
install -m 755 "$base/otp" "$base/otpc" "$dest/"
# Wrapper scripts rather than symlinks: on macOS std::env::current_exe()
# returns the symlink path (not the resolved target), so the compiler's
# runtime resolver cannot find libotp_rtl.a next to the real binary via a
# bin -> lib symlink. The wrapper sets OTP_RUNTIME explicitly and execs
# the installed binary, keeping bin/ clean of large archives.
for name in otp otpc; do
 wrapper="$prefix/bin/$name"
 # Quote dest with single-quote escaping so paths with spaces or quotes work.
 esc_dest=$(printf '%s' "$dest" | sed "s/'/'\\\\''/g")
 cat >"$wrapper" <<EOF
#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
export OTP_RUNTIME='$esc_dest/libotp_rtl.a'
exec '$esc_dest/$name' "\$@"
EOF
 chmod 755 "$wrapper"
done
app_src="$base/ODIN Turbo Pascal.app"
app_dst="$app_prefix/ODIN Turbo Pascal.app"
if [ -d "$app_src" ]; then
 rm -rf "$app_dst"
 cp -R "$app_src" "$app_dst"
 # Strip the quarantine attribute Gatekeeper adds when the DMG was downloaded,
 # so an ad-hoc signed development build opens without a right-click first.
 xattr -dr com.apple.quarantine "$app_dst" 2>/dev/null || true
 # Re-sign ad-hoc if the signature was damaged by the copy.
 if command -v codesign >/dev/null 2>&1; then
  codesign --verify --deep "$app_dst" >/dev/null 2>&1 || \
   codesign --force --deep --sign - "$app_dst" >/dev/null 2>&1 || true
 fi
fi
printf 'Installed ODIN Turbo Pascal to %s\n' "$dest"
if [ -d "$app_dst" ]; then
 printf 'Application bundle: %s\n' "$app_dst"
fi
printf 'Run: %s/bin/otp\nCLI: %s/bin/otpc doctor\n' "$prefix" "$prefix"
case ":${PATH:-}:" in
 *":$prefix/bin:"*) ;;
 *) printf 'Add this to your shell profile to use otpc from anywhere:\n  export PATH="%s/bin:$PATH"\n' "$prefix" ;;
esac
if ! xcode-select -p >/dev/null 2>&1; then
 printf 'Compilation needs the Xcode Command Line Tools (xcode-select --install).\n'
fi
