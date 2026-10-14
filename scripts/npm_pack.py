#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Build the npm tarballs for ODIN Turbo Pascal and smoke-test installing them.

  npm_pack.py platform [--target-dir target/release] --out DIR   # host binaries package
  npm_pack.py main --out DIR                                      # launcher package
  npm_pack.py smoke TARBALL...                                    # offline install + run HELLO.PAS
  npm_pack.py version                                             # print the checked release version
"""
import argparse, io, json, os, platform, re, shutil, subprocess, sys, tarfile, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MAIN = ROOT / 'npm' / 'odin-turbo-pascal'
CFG = json.loads((MAIN / 'lib' / 'platforms.json').read_text())
META = json.loads((MAIN / 'package.json').read_text())
NPM_EPOCH = 499162500  # fixed mtime npm itself uses inside tarballs


def workspace_version():
    section = re.search(r'^\[workspace\.package\]$(.*?)(?=^\[|\Z)', (ROOT / 'Cargo.toml').read_text(), re.M | re.S)
    return re.search(r'^version\s*=\s*"([^"]+)"', section.group(1), re.M).group(1)


def checked_version():
    version = workspace_version()
    expected = {f"{CFG['scope']}/{key}": version for key in sorted(CFG['platforms'])}
    if META['version'] != version or META.get('optionalDependencies') != expected:
        sys.exit(f'npm/odin-turbo-pascal/package.json must use version {version} for itself and every '
                 f'optionalDependency (found {META["version"]}).')
    return version


def host_key():
    system = {'Darwin': 'darwin', 'Linux': 'linux', 'Windows': 'win32'}.get(platform.system())
    arch = {'x86_64': 'x64', 'amd64': 'x64', 'arm64': 'arm64', 'aarch64': 'arm64'}.get(platform.machine().lower())
    key = f'{system}-{arch}'
    if key not in CFG['platforms']:
        sys.exit(f'No npm platform package for {platform.system()} {platform.machine()}')
    return key


def write_tgz(out, name, version, entries):
    out.mkdir(parents=True, exist_ok=True)
    path = out / (name.lstrip('@').replace('/', '-') + f'-{version}.tgz')
    with tarfile.open(path, 'w:gz') as tar:
        for arcname, data, mode in entries:
            info = tarfile.TarInfo('package/' + arcname)
            info.size, info.mode, info.mtime = len(data), mode, NPM_EPOCH
            tar.addfile(info, io.BytesIO(data))
    print(path)
    return path


def manifest_bytes(manifest):
    return (json.dumps(manifest, indent=2) + '\n').encode()


def cmd_platform(args):
    version = checked_version()
    key = host_key()
    spec = CFG['platforms'][key]
    name = f"{CFG['scope']}/{key}"
    manifest = {
        'name': name,
        'version': version,
        'description': f"ODIN Turbo Pascal binaries for {spec['title']} (installed automatically by odin-turbo-pascal)",
        **{k: META[k] for k in ('license', 'author', 'homepage', 'repository', 'bugs')},
        'os': [spec['os']],
        'cpu': [spec['cpu']],
        **({'libc': [spec['libc']]} if 'libc' in spec else {}),
        'publishConfig': {'access': 'public'},
    }
    readme = (f"# {name}\n\nNative `otp` and `otpc` binaries of ODIN Turbo Pascal for {spec['title']}.\n"
              "Do not install this package directly; install the launcher instead:\n\n"
              "```sh\nnpm install -g odin-turbo-pascal\n```\n")
    target = Path(args.target_dir)
    entries = [('package.json', manifest_bytes(manifest), 0o644),
               ('README.md', readme.encode(), 0o644),
               ('LICENSE', (ROOT / 'LICENSE').read_bytes(), 0o644)]
    for tool in ('otp', 'otpc'):
        exe = target / (tool + spec['exe'])
        if not exe.is_file():
            sys.exit(f'Missing {exe}; run cargo build --workspace --release first')
        entries.append((f"bin/{tool}{spec['exe']}", exe.read_bytes(), 0o755))
    runtime = target / spec['runtime']
    if not runtime.is_file():
        sys.exit(f'Missing {runtime}; run cargo build --workspace --release first')
    entries.append((f"lib/{spec['runtime']}", runtime.read_bytes(), 0o644))
    write_tgz(Path(args.out), name, version, entries)


def cmd_main(args):
    version = checked_version()
    entries = [('package.json', (MAIN / 'package.json').read_bytes(), 0o644),
               ('README.md', (MAIN / 'README.md').read_bytes(), 0o644),
               ('LICENSE', (ROOT / 'LICENSE').read_bytes(), 0o644)]
    entries += [(f'bin/{p.name}', p.read_bytes(), 0o755) for p in sorted((MAIN / 'bin').iterdir())]
    entries += [(f'lib/{p.name}', p.read_bytes(), 0o644) for p in sorted((MAIN / 'lib').iterdir())]
    write_tgz(Path(args.out), META['name'], version, entries)


def cmd_smoke(args):
    npm, node = shutil.which('npm'), shutil.which('node')
    if not npm or not node:
        sys.exit('npm and node are required for the smoke test')
    tarballs = [str(Path(t).resolve()) for t in args.tarballs]
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        (work / 'package.json').write_text('{"name": "otp-npm-smoke", "private": true}\n')
        # Offline: the other platforms' optional packages are not published yet and must be skipped.
        subprocess.run([npm, 'install', '--offline', '--no-audit', '--no-fund', *tarballs], cwd=work, check=True)
        shim = work / 'node_modules' / '.bin' / ('otpc.cmd' if os.name == 'nt' else 'otpc')
        if not shim.exists():
            sys.exit(f'npm did not create the otpc bin shim at {shim}')
        launcher = work / 'node_modules' / 'odin-turbo-pascal' / 'bin' / 'otpc.js'
        subprocess.run([node, str(launcher), 'doctor'], check=True)
        hello = work / 'HELLO.PAS'
        shutil.copy2(ROOT / 'examples' / 'HELLO.PAS', hello)
        run = subprocess.run([node, str(launcher), 'run', str(hello)], capture_output=True, text=True)
        print(run.stdout + run.stderr)
        if run.returncode != 0 or 'Selamat datang di ODIN Turbo Pascal' not in run.stdout:
            sys.exit('npm smoke test failed: otpc run HELLO.PAS did not produce the expected output')
    print('npm smoke test passed')


parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
sub = parser.add_subparsers(dest='command', required=True)
p = sub.add_parser('platform')
p.add_argument('--target-dir', default=str(ROOT / 'target' / 'release'))
p.add_argument('--out', required=True)
p.set_defaults(func=cmd_platform)
p = sub.add_parser('main')
p.add_argument('--out', required=True)
p.set_defaults(func=cmd_main)
p = sub.add_parser('smoke')
p.add_argument('tarballs', nargs='+')
p.set_defaults(func=cmd_smoke)
p = sub.add_parser('version')
p.set_defaults(func=lambda _: print(checked_version()))
args = parser.parse_args()
args.func(args)
