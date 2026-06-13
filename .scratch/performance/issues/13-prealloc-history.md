# 13 — Pre-allocate Board history capacity

**Status**: `completed`
**Category**: `enhancement`

## Problem

`Board::history` is `Vec<u64>`. `make_move` calls `self.history.push(self.hash)` every move. At deep search with many iterations, the Vec may reallocate. The clone path also clones the Vec (heap allocation).

## Solution

- In `Board::new()` / `Board::from_fen()`, pre-allocate history capacity (e.g., 256 entries):
```rust
self.history.reserve(256);
```
- In `Board::clone()`, reserve the same capacity on the clone:
```rust
fn clone(&self) -> Self {
    // ...
    cloned.history.reserve(self.history.capacity());
    // ...
}
```

256 entries covers a full game + search depth comfortably (max game length is ~200 halfmoves + search depth).

## Acceptance criteria

- [ ] `history` has pre-allocated capacity (no reallocation during search)
- [ ] All 86 unit tests pass
- [ ] All 10 tactical tests pass
- [ ] No functional change

## Comments
