# 03 — Eliminate the DEFAULT_EVAL global

**Status:** `completed`

**Status**: `completed`
**Category**: `enhancement`

## Problem

A `static DEFAULT_EVAL: OnceLock<Eval>` global singleton hides the evaluation dependency from search. Search calls the free function `eval::evaluate(board)` which reads the global. Tests cannot inject a custom Eval without swapping the global — a thread-safety hazard and a testability blocker.

## What to change

1. Remove `static DEFAULT_EVAL` and the `pub fn evaluate(board) -> i32` free function that uses it.
2. Make search accept `&Eval` explicitly:
   - `search()` takes `eval: &Eval`
   - `search_single()` passes it through
   - `search_mt()` clones it per worker or wraps in `Arc`
3. Update all call sites:
   - UCI: constructs `Eval::default()` and passes it to search
   - Benchmarks: construct Eval explicitly
   - Perft / integration tests: construct Eval explicitly
4. Keep the `evaluate()` free function but rename to `evaluate_with_default()` or remove entirely — the convenience can be re-added as `Eval::evaluate_default(board)` if needed.

## Key interfaces

- `search()` signature changes: `search(board, params, stop, tt, eval: &Eval) -> SearchResult`
- `search_single()` — same
- `search_mt()` — clones Eval per thread (Eval is already Clone)
- `Eval::default()` — already exists, use directly instead of global
- Free function `evaluate()` — removed or kept as thin wrapper

## Acceptance criteria

- [ ] No `static` or `OnceLock` in eval.rs's public API
- [ ] Search takes `&Eval` explicitly
- [ ] All 86 unit tests pass
- [ ] All 10 tactical integration tests pass
- [ ] Benchmarks pass (explicit Eval construction)
- [ ] UCI `go` command works without the global
- [ ] No NPS regression

## Out of scope

- Splitting eval.rs into sub-modules (issue #02)
- Changing the Eval struct interface (issue #05)
- Thread-local evaluation caches

## Comments
