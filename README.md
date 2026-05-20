# screenshot-mcp

[![npm](https://img.shields.io/npm/v/@kembec/screenshot-mcp)](https://www.npmjs.com/package/@kembec/screenshot-mcp)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Capture full-page and viewport screenshots of any URL via Chrome DevTools Protocol — single static binary, no runtime required.

## Prerequisites

No manual setup required. On first use, `screenshot-mcp` automatically downloads a pinned Chromium build to your OS cache directory (`~/.cache/screenshot-mcp/` on Linux, `~/Library/Caches/screenshot-mcp/` on macOS) if no Chrome is detected.

If you already have Chrome installed, it is used directly — no download occurs. You can also set `CHROME_PATH` to point to any Chromium-compatible binary.

## Installation

```bash
npm install -g @kembec/screenshot-mcp
```

## Configuration

### Cursor

Add to `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "screenshot": {
      "command": "npx",
      "args": ["-y", "@kembec/screenshot-mcp"]
    }
  }
}
```

Chrome is auto-downloaded on first use if not found. To override: `"env": { "CHROME_PATH": "/path/to/chrome" }`.

### Claude Desktop

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "screenshot": {
      "command": "npx",
      "args": ["-y", "@kembec/screenshot-mcp"]
    }
  }
}
```

### Codex CLI

Add to `~/.codex/config.toml`:

```toml
[mcp_servers.screenshot]
command = "npx"
args = ["-y", "@kembec/screenshot-mcp"]
enabled = true
```

## Tools

### `capture_page`

Captures a screenshot of a web page, waiting for JavaScript to finish rendering.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `url` | string | required | URL to capture (http/https) |
| `viewport` | string | `desktop` | `desktop` (1440×900), `laptop` (1280×800), `tablet` (768×1024), `mobile` (390×844 + mobile UA), `mobile_landscape`, `custom` |
| `full_page` | boolean | `true` | Capture full scrollable height |
| `wait_strategy` | string | `networkidle` | `networkidle`, `load`, `domcontentloaded`, `delay:N` |
| `format` | string | `png` | `png` or `jpeg` |
| `output_path` | string | — | Save to file path; if omitted, image is returned inline |
| `width` | integer | — | Required when `viewport` is `custom` |
| `height` | integer | — | Required when `viewport` is `custom` |

## Building from source

```bash
cargo build --release
./target/release/screenshot-mcp
```

## License

MIT
