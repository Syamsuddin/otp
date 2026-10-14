// SPDX-License-Identifier: GPL-3.0-only
'use strict';
const { spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const { scope, platforms } = require('./platforms.json');

function fail(message) {
  process.stderr.write(`odin-turbo-pascal: ${message}\n`);
  process.exit(1);
}

function locatePlatformPackage() {
  const key = `${process.platform}-${process.arch}`;
  const spec = platforms[key];
  if (!spec) {
    fail(`no prebuilt binaries for ${key}. Supported: ${Object.keys(platforms).join(', ')}.`);
  }
  const name = `${scope}/${key}`;
  try {
    return { spec, dir: path.dirname(require.resolve(`${name}/package.json`)) };
  } catch (_) {
    return fail(
      `platform package ${name} is not installed. Reinstall without --omit=optional:\n` +
        '  npm install -g odin-turbo-pascal'
    );
  }
}

module.exports = function launch(tool) {
  const { spec, dir } = locatePlatformPackage();
  const exe = path.join(dir, 'bin', tool + spec.exe);
  if (!fs.existsSync(exe)) fail(`missing ${exe}; the platform package looks incomplete.`);
  const env = Object.assign({}, process.env);
  if (!env.OTP_RUNTIME) env.OTP_RUNTIME = path.join(dir, 'lib', spec.runtime);
  const result = spawnSync(exe, process.argv.slice(2), { stdio: 'inherit', env });
  if (result.error) fail(`cannot start ${exe}: ${result.error.message}`);
  if (result.signal) process.kill(process.pid, result.signal);
  process.exit(result.status === null ? 1 : result.status);
};
