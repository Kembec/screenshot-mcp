#!/usr/bin/env node
const { execFileSync } = require('child_process');
const { existsSync, chmodSync } = require('fs');

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
  process.exit(0);
}

const binName = process.platform === 'win32' ? 'screenshot-mcp.exe' : 'screenshot-mcp';

let binPath;
try {
  binPath = require.resolve(`${pkg}/bin/${binName}`);
} catch {
  process.exit(0);
}

if (!existsSync(binPath)) {
  process.exit(0);
}

try { chmodSync(binPath, 0o755); } catch {}

try {
  execFileSync(binPath, ['--prefetch-chrome'], { stdio: 'inherit' });
} catch {
  process.stderr.write('screenshot-mcp: Chrome prefetch failed, will retry on first use\n');
}
