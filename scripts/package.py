#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Package binaries built and tested on the current host. Does not cross-compile."""
import argparse, hashlib, json, os, platform, plistlib, shutil, subprocess, tarfile, zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
VERSION='2.0.0-dev.2'
parser=argparse.ArgumentParser();parser.add_argument('--skip-build',action='store_true');args=parser.parse_args()
if not args.skip_build:
 subprocess.run(['cargo','build','--workspace','--release','--locked'],cwd=ROOT,check=True)
 subprocess.run(['cargo','test','--workspace','--locked'],cwd=ROOT,check=True)
 subprocess.run([os.sys.executable,'tests/native_suite.py','--release'],cwd=ROOT,check=True)
system=platform.system();arch=platform.machine().lower()
if system=='Darwin':label='macos-arm64' if arch=='arm64' else 'macos-x64'
elif system=='Windows':label='windows-x64'
elif system=='Linux' and arch=='x86_64':label='linux-x64'
else:raise SystemExit('Unsupported packaging host: '+system+' '+arch)
dist=ROOT/'dist';stage=dist/f'OTP-{VERSION}-{label}';stage.mkdir(parents=True,exist_ok=True)
release=ROOT/'target'/'release';ext='.exe' if system=='Windows' else ''
for n in ['otp','otpc']:
 src=release/(n+ext)
 if not src.exists():raise SystemExit(f'Missing {src}; run cargo build --workspace --release')
 shutil.copy2(src,stage/(n+ext))
libs=[p for p in [release/'libotp_rtl.a',release/'otp_rtl.lib'] if p.exists()]
if not libs:raise SystemExit('Missing native Rust runtime archive')
shutil.copy2(libs[0],stage/libs[0].name)
for name in ['LICENSE','README.md','THIRD_PARTY.md']:shutil.copy2(ROOT/name,stage/name)
for name in ['docs','examples','assets','third-party-licenses']:shutil.copytree(ROOT/name,stage/name,dirs_exist_ok=True)
report=ROOT/'target'/'native-test-report-release.json'
if report.exists():shutil.copy2(report,stage/'docs'/'native-test-report.json')
shutil.copy2(ROOT/'packaging'/'install-linux.sh',stage/'install-linux.sh')
shutil.copy2(ROOT/'packaging'/'install-windows.ps1',stage/'install-windows.ps1')
shutil.copy2(ROOT/'packaging'/'install-macos.sh',stage/'install-macos.sh')
metadata={'version':VERSION,'platform':label,'build_host':platform.platform(),'status':'development subset, not full TP7 compatibility','external_linker_required':True,'runtime':libs[0].name}
(stage/'BUILD.json').write_text(json.dumps(metadata,indent=2))
if system=='Darwin':
 app=stage/'ODIN Turbo Pascal.app';contents=app/'Contents';binary=contents/'MacOS';resources=contents/'Resources';binary.mkdir(parents=True,exist_ok=True);resources.mkdir(exist_ok=True)
 for n in ['otp','otpc']:shutil.copy2(stage/n,binary/n)
 (resources/'lib').mkdir(exist_ok=True);shutil.copy2(libs[0],resources/'lib'/libs[0].name)
 for n in ['docs','examples','assets']:shutil.copytree(stage/n,resources/n,dirs_exist_ok=True)
 info={'CFBundleName':'ODIN Turbo Pascal','CFBundleDisplayName':'ODIN Turbo Pascal','CFBundleIdentifier':'org.odinpascal.otp','CFBundleExecutable':'otp','CFBundlePackageType':'APPL','CFBundleVersion':'2.0.0','CFBundleShortVersionString':VERSION,'NSHighResolutionCapable':True,'LSMinimumSystemVersion':'11.0','CFBundleDocumentTypes':[{'CFBundleTypeExtensions':['pas'],'CFBundleTypeName':'Pascal Source','CFBundleTypeRole':'Editor'}]}
 with (contents/'Info.plist').open('wb') as f:plistlib.dump(info,f)
 # Ad-hoc signing is for local development. Developer ID and notarization are separate release gates.
 subprocess.run(['codesign','--force','--deep','--sign','-',str(app)],check=True)
 dmg=dist/(stage.name+'.dmg')
 subprocess.run(['hdiutil','create','-volname','ODIN Turbo Pascal','-srcfolder',str(stage),'-ov','-format','UDZO',str(dmg)],check=True)
if system=='Windows':
 archive=dist/(stage.name+'.zip')
 with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED) as z:
  for p in sorted(stage.rglob('*')):
   if p.is_file():z.write(p,p.relative_to(dist))
else:
 archive=dist/(stage.name+'.tar.gz')
 with tarfile.open(archive,'w:gz') as t:t.add(stage,arcname=stage.name)
checksum=hashlib.sha256(archive.read_bytes()).hexdigest();(dist/(archive.name+'.sha256')).write_text(f'{checksum}  {archive.name}\n')
print(archive)
