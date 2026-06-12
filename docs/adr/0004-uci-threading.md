# Two-thread UCI architecture (I/O thread + search threads) with Lazy SMP, Ponder, and MultiPV

**Status**: accepted

## Context

The UCI protocol uses stdin/stdout for communication while the search loop is a CPU-bound tight loop. These two activities cannot share a single thread without explicit cooperation (polling stdin during search, which is fragile and platform-specific).

## Decision

The engine uses a **1 + N thread model**:

1. **Main/I/O thread**: reads stdin line-by-line, parses UCI commands, dispatches handlers. Spawns and joins search threads. Owns the engine state (board, TT, stop flag, book, pondering flags).
2. **N search threads** (Lazy SMP): spawned on `go` command. All threads run the same iterative-deepening alpha-beta search on the same position. They share a lock-free transposition table via `Arc<TT>` and a single `Arc<AtomicBool>` stop flag. Thread 0 is the authoritative reporter; threads 1..N provide search diversity.

The I/O thread sets the stop flag on receiving `stop` and joins all search threads before printing `bestmove`.

### Ponder

`go ponder` starts a speculative search during the opponent's turn (infinite mode, no movetime timer). `ponderhit` clears the pondering flag so the search result is reported normally. `stop` without `ponderhit` silently discards the ponder search. Pondering state is tracked via `Engine.pondering: bool` and `ponderhit_received: Arc<AtomicBool>` shared with the search thread.

### MultiPV

When `MultiPV > 1`, each depth iteration runs a MultiPV loop: N sequential searches with per-index aspiration windows and excluded-move lists (previously found best moves are skipped). Results are reported as `info multipv N ...` lines.

### Opening book

If `OwnBook` is enabled and a book file is loaded, `cmd_go` probes the book before spawning any search threads. On a hit, `bestmove` is emitted immediately and the search is skipped entirely.

## Multi-threaded search (Lazy SMP)

Lazy SMP is the standard multi-threading approach for modern chess engines. It is "lazy" because threads operate independently without explicit work partitioning — they cooperate implicitly through the shared transposition table.

### Thread diversity

To prevent all threads from searching identical trees (which would waste cores), each thread perturbs its root move ordering:

- **Thread 0**: normal move ordering (hash move first, captures by SEE, promotions, killers, history heuristic, losing captures last).
- **Thread i (i > 0)**: quiet moves receive a score perturbation = `(from_square × thread_id) % 16`. Hash moves and capture scores are untouched.

### Result collection

- Each thread runs independently and returns a `SearchResult`.
- The best result across threads (highest depth completed, then deepest PV) is reported.
- Thread 0's result is authoritative for the reported best move.

## Why

- **I/O separation**: Keeps blocking I/O off the compute thread.
- **Lazy SMP**: The simplest multi-threading model. No work-stealing, no split-point handling, no complex synchronization. ~50 lines of code.
- **TT as implicit work distributor**: Threads deposit evaluation data into the TT. Other threads read it. This creates natural work sharing without explicit coordination.
- **`Arc<AtomicBool>`**: Zero-cost stop signalling. Shared across all threads.

## Considered options

| Option | Rejected because |
|--------|------------------|
| Single-threaded with non-blocking stdin poll | Platform-specific (works on Linux with `fcntl`, fails on Windows). Search must voluntarily yield to check stdin. |
| Async with Tokio | Adds a heavy dependency. The concurrency model here is trivial (spawn threads, wait for them). |
| Message-passing via channels | More machinery (mpsc channels) for the same outcome. `Arc<AtomicBool>` is simpler and faster. |
| YBWC / DTS (work-stealing parallel search) | Massive code complexity (~500+ lines). Lazy SMP achieves the TT-sharing benefit with 5% of the code. |

## Consequences

- Each search thread operates on its own cloned `Board` (make/unmake in-place, no shared mutable state per thread).
- `SearchParams` is `Clone` and passed by value to each search thread.
- On `quit`, the engine must join all search threads before exiting to avoid detached threads.
- TT is shared via `Arc<TT>` with lock-free atomics. Implementation details in `src/tt.rs` and ADR-0002.
- Scaling data lives in `CONTEXT.md` under "Lazy SMP scaling data."
