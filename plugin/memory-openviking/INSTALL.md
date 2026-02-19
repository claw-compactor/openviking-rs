# Installation

## 1. Build the native module
```bash
cd crates/ov-napi
npm run build  # or: cargo build --release -p ov-napi && napi build --release
```

This produces `openviking-engine.darwin-arm64.node` (or equivalent for your platform).

## 2. Install the plugin
```bash
mkdir -p ~/.openclaw/extensions/memory-openviking
cp plugin/memory-openviking/* ~/.openclaw/extensions/memory-openviking/
cp crates/ov-napi/openviking-engine.darwin-arm64.node ~/.openclaw/extensions/memory-openviking/
```

## 3. Configure OpenClaw
Add to `~/.openclaw/openclaw.json`:
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

## 4. Restart gateway
```bash
openclaw gateway restart
```

## Verify
```bash
openclaw plugins list
# Should show: memory-openviking — loaded, memory-core — disabled
```
