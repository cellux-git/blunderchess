# 09 — Encapsulate Engine: private fields, fix ThreadPool lifecycle

**Status:** `completed`

**Category:** improvement
****Status:** completed
**ADR:** docs/adr/0009-engine-facade.md

## Summary

The `Engine` struct has all 11 fields `pub` (or `pub(crate)`). Any module can mutate `board` directly, bypassing the UCI protocol. The ThreadPool is replaced at runtime (`cmd_go_impl` line 257), leaking old worker threads behind an `Arc`. Make all fields private and expose only two public methods + constructor.

## Current leak

```rust
// src/uci.rs, cmd_go_impl
self.pool = Arc::new(ThreadPool::new(needed));
// old pool's workers live until the Arc is dropped — which may be never
```

## Desired interface

- `Engine::new()` — unchanged
- `Engine::process_command(&mut self, line: &str) -> bool` — unchanged (already public)
- `Engine::search_position(&mut self, board: &Board, depth: u8) -> SearchResult` — new, test-only entry point

## Acceptance criteria

- [ ] All 11 `Engine` fields made private (struct-level, not per-field)
- [ ] `ThreadPool` resized via drain+join+respawn instead of Arc replacement
- [ ] Integration tests in `tests/benchmarks.rs` use `engine.search_position()`  
- [ ] `start_async_search` becomes a private method
- [ ] `cargo test --lib --test benchmarks` passes cleanly
- [ ] UCI `go` with changing thread counts does not leak threads
- [ ] No NPS regression
