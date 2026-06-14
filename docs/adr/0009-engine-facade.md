# Engine facade with private state

**Status**: accepted

## Context

The `Engine` struct in `src/uci.rs` has all 11 fields declared `pub` (or `pub(crate)`): `board`, `tt`, `pool`, `stop_flag`, `search_handles`, `multi_pv`, `pondering`, `ponderhit_received`, `book`, `own_book`, `threads`. The UCI command handlers mutate these fields directly, but any module in the crate can also mutate them, bypassing the UCI protocol entirely.

Additionally, `cmd_go_impl` replaces the `ThreadPool` at runtime when the requested thread count exceeds the pool size:
```rust
self.pool = Arc::new(ThreadPool::new(needed));
```
The old pool's worker threads are only cleaned up on `Drop`, but the `Arc` may keep them alive if any pending work references it.

The Engine is described in CONTEXT.md as a "facade" but behaves as a god object — it owns and directly manipulates every subsystem rather than delegating to sub-facades.

## Decision

Make all `Engine` fields private. Expose only:

- `Engine::new()` — constructs with initial board, 64MB TT, thread pool
- `Engine::process_command(&mut self, line: &str) -> bool` — UCI command dispatcher (already public)
- `Engine::search_position(&mut self, board: &Board, depth: u8) -> SearchResult` — test-only entry point for integration tests

The `ThreadPool` lifecycle is managed internally: when a `go` command requests more threads than the pool has, the engine resizes the pool by draining pending work, joining old workers, and spawning new ones — rather than replacing the `Arc`.

The `Board` field is only mutated through `cmd_position` and `process_command` dispatching. No other module touches it.

## Why

- **Encapsulation**: the Engine's internal state is an implementation detail. Callers interact through `process_command` only.
- **ADR-0004 compliant**: the two-thread UCI architecture (I/O thread + search threads) is preserved. This ADR only tightens the Engine's interface, not the threading model.
- **Locality**: ThreadPool lifecycle bug is fixed in one place (the Engine) rather than leaking across `cmd_go_impl` and search worker lifetimes.
- **Leverage**: one public method (`process_command`) gates all UCI behavior. Tests use `search_position` as a narrow secondary seam.

## Considered options

| Option | Rejected because |
|--------|------------------|
| All pub fields (current) | No encapsulation. ThreadPool leak. Any module can mutate board state. |
| Trait-based `UciEngine` + `SearchEngine` | One adapter (the only Engine implementation) = hypothetical seam. A trait would be indirection without benefit until a second adapter exists. |
| Move Engine to lib.rs as a proper facade | Would couple `lib.rs` to uci/search/eval. The Engine lives at the UCI layer because it owns UCI-specific state (pondering, multi_pv, async search handles). |

## Consequences

- Integration tests in `tests/benchmarks.rs` call `engine.search_position(board, depth)` instead of constructing raw `search::search()` calls.
- `start_async_search` becomes a private method; async search lifecycle is opaque to callers.
- `ThreadPool::resize()` may need to be added (or pool size fixed at construction with sufficient capacity).
- No behavior change: the UCI protocol works identically.
