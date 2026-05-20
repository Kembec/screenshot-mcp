#!/usr/bin/env node
const { execFileSync } = require('child_process');

const PLATFORMS = {
  'darwin-arm64': '@kembec/screenshot-mcp-darwin-arm64',
  'darwin-x64':   '@kembec/screenshot-mcp-darwin-x64',
  'linux-x64':    '@kembec/screenshot-mcp-linux-x64',
  'linux-arm64':  '@kembec/screenshot-mcp-linux-arm64',
  'win32-x64':    '@kembec/screenshot-mcp-win32-x64',
};

const key = `${process.platform}-${process.arch}`;
const pkg = PLATFORMS[key];
if (!pkg) {
  console.error(`screenshot-mcp: unsupported platform ${key}`);
  process.exit(1);
}

const binName = process.platform === 'win32' ? 'screenshot-mcp.exe' : 'screenshot-mcp';

let binPath;
try {
  binPath = require.resolve(`${pkg}/bin/${binName}`);
} catch {
  console.error(`screenshot-mcp: platform package ${pkg} is not installed.`);
  console.error('Reinstall with `npm install @kembec/screenshot-mcp` to pick the right binary.');
  process.exit(1);
}

try { require('fs').chmodSync(binPath, 0o755); } catch {}

try {
  execFileSync(binPath, process.argv.slice(2), { stdio: 'inherit' });
} catch (e) {
  process.exit(typeof e.status === 'number' ? e.status : 1);
}
