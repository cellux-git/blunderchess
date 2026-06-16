# 12 — Pre-allocate excluded_moves Vec in SearchState

**Status:** `completed`

**Status**: `completed`
**Category**: `enhancement`

## Problem

In `search_worker`, `excluded_moves: Vec<Move>` is created fresh each depth iteration (`search.rs ~171`), then cloned into `SearchState` at line ~178. In single-PV mode (the common case), this is an empty Vec — overhead is minimal. In multi-PV mode with N lines, it allocates per iteration.

## Solution

Move `excluded_moves` into `SearchState` as a persistent Vec. Clear and reuse it each iteration instead of allocating:

```rust
// In SearchState:
excluded_moves: Vec<Move>,

// In search_worker loop:
state.excluded_moves.clear();
```

Pre-allocate capacity on first use:
```rust
state.excluded_moves.reserve(multi_pv as usize);
```

## Acceptance criteria

- [ ] `excluded_moves` is a persistent field on `SearchState`, not allocated per iteration
- [ ] `.clear()` called at the start of each PV iteration
- [ ] Multi-PV search behavior unchanged
- [ ] All 13 search tests pass
- [ ] MultiPV UCI test passes

## Comments
