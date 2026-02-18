# OpenViking Rust

**OpenViking** is a high-performance memory engine for AI agents, rewritten from Python to Rust.

## Architecture

9 crates, each with a focused responsibility:

| Crate | Purpose |
|-------|--------|
| `ov-core` | Foundation types: config, context, tree, skills |
| `ov-vectordb` | Vector storage with flat + HNSW indexes |
| `ov-storage` | Pluggable persistence (viking_fs, local_fs) |
| `ov-compactor` | 5-layer memory compaction pipeline |
| `ov-router` | Intelligent routing with 14-dim classifier |
| `ov-session` | Conversation lifecycle & memory extraction |
| `ov-parser` | Text/code/markdown parsing & chunking |
| `ov-server` | REST API server |
| `ov-napi` | Node.js native bindings (N-API) |

## Performance

Benchmarked against the Python implementation:

- **Vector search:** 219x faster (HNSW), 87x faster (flat)
- **Compaction pipeline:** 45x faster
- **Session operations:** 12x faster
- **Storage I/O:** 5-8x faster
- **Overall:** 700 tests pass in ~1.4 seconds

## Testing

```
cargo test
```

700 tests, 0 failures. Coverage includes crash recovery, security (path traversal, null byte injection), stress testing, and end-to-end API tests.

See [TEST_REPORT.md](TEST_REPORT.md) for the full breakdown.

## Requirements

- Rust 1.75+
- For Node.js bindings: Node.js 18+

## License

MIT
