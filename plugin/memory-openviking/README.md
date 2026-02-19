# memory-openviking — OpenViking-rs Memory Plugin

## What it does
Replaces OpenClaw's built-in memory-core plugin with OpenViking-rs (Rust native engine).

## How it works
1. Plugin is configured as `plugins.slots.memory = "memory-openviking"`
2. This disables memory-core (built-in) and loads this plugin instead
3. Plugin registers `memory_search` and `memory_get` tools
4. Uses OpenViking-rs native module for search, with file-based fallback

## Files
- `index.ts` — Plugin source
- `openviking-engine.darwin-arm64.node` — Native Rust module (NAPI)
- `openclaw.plugin.json` — Plugin manifest
- `package.json` — Package metadata

## Update Survival
This plugin lives in `~/.openclaw/extensions/` which is NOT touched by `openclaw update`.
The plugin system is the official extension mechanism — no source patches needed.

## Config (in openclaw.json)
```json
{
  "plugins": {
    "enabled": true,
    "slots": {
      "memory": "memory-openviking"
    },
    "installs": {
      "memory-openviking": {
        "path": "~/.openclaw/extensions/memory-openviking"
      }
    }
  }
}
```
