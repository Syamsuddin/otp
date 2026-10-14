#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
[ "$(uname -s)" = Linux ] || { echo 'This installer is for Linux.' >&2; exit 1; }
[ "$(uname -m)" = x86_64 ] || { echo 'This build requires x86_64.' >&2; exit 1; }
base=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
prefix=${OTP_PREFIX:-"$HOME/.local"}
dest="$prefix/lib/odin-turbo-pascal/2.0.0-dev.3"
for f in otp otpc libotp_rtl.a; do
 [ -f "$base/$f" ] || { echo "Missing package file: $f" >&2; exit 1; }
done
mkdir -p "$dest" "$prefix/bin" "$prefix/share/applications" "$prefix/share/icons/hicolor/512x512/apps"
cp -R "$base/docs" "$base/examples" "$base/assets" "$base/third-party-licenses" "$dest/"
cp "$base/LICENSE" "$base/README.md" "$base/THIRD_PARTY.md" "$base/BUILD.json" "$base/libotp_rtl.a" "$dest/"
install -m 755 "$base/otp" "$base/otpc" "$dest/"
# Relative symlinks also work when the installation directory contains spaces.
ln -sfn ../lib/odin-turbo-pascal/2.0.0-dev.3/otp "$prefix/bin/otp"
ln -sfn ../lib/odin-turbo-pascal/2.0.0-dev.3/otpc "$prefix/bin/otpc"
cp "$base/assets/branding/odin-turbo-pascal-master-v1.png" "$prefix/share/icons/hicolor/512x512/apps/odin-turbo-pascal.png"
# Desktop entry escapes backslashes, double quotes, backticks and dollar signs in paths.
python3 - "$prefix" <<'PY'
from pathlib import Path
import sys
p=Path(sys.argv[1]).resolve()
def escape(s):
 return str(s).replace('\\','\\\\').replace('"','\\"').replace('`','\\`').replace('$','\\$')
entry='[Desktop Entry]\nType=Application\nName=ODIN Turbo Pascal 2.0 (development)\nComment=Native Pascal IDE written in Rust\nExec="'+escape(p/'bin/otp')+'"\nIcon='+str(p/'share/icons/hicolor/512x512/apps/odin-turbo-pascal.png')+'\nTerminal=false\nCategories=Development;IDE;\n'
(p/'share/applications/odin-turbo-pascal.desktop').write_text(entry)
PY
printf 'Installed ODIN Turbo Pascal to %s\n' "$dest"
printf 'Run: %s/bin/otp\nCLI: %s/bin/otpc doctor\n' "$prefix" "$prefix"
printf 'Compilation requires a native C linker (cc), usually provided by build-essential.\n'
