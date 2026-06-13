# 09 — Thread pool for search workers

**Status**: `ready-for-agent`
**Category**: `enhancement`

## Problem

`search_mt()` spawns new OS threads via `std::thread::spawn` for every `go` command, then joins them. Thread creation costs ~0.1-1ms per thread. With rapid `go`/`stop` cycles (e.g., fast bullet games), this overhead accumulates.

## Solution

Create a persistent thread pool that lives for the engine's lifetime. Options:

**A) Simple fixed pool:** Spawn N threads at `Engine` construction. Use `std::sync::mpsc` to send work items. Threads park waiting for work. On `stop`, signal and wait. On shutdown, send poison pill.

**B) Use `rayon`:** Add `rayon` as a dependency (lightweight, widely used). Replace `std::thread::spawn` with `rayon::spawn`. Rayon handles pooling and work-stealing automatically.

Recommend option A for zero-dependency, option B for simplicity. The current zero-dependency ADR allows exceptions for widely-used crates (like `log`).

## Acceptance criteria

- [ ] Threads created once at engine startup, reused across searches
- [ ] `std::thread::spawn` no longer called in search hot path
- [ ] All 86 unit tests pass
- [ ] All 10 tactical tests pass
- [ ] Multi-threaded search test passes
- [ ] No regression in `go`/`stop` latency

## Comments
