# 05 — Unify SEE piece values with Eval

**Status**: `completed`
**Category**: `enhancement`

## Problem

`see_piece_value()` in eval.rs duplicates the same piece values as `Eval::material_value()` as hardcoded constants. If a tuner changes `Eval::knight_value` from 320 to 310, SEE still uses 320 — causing move ordering to disagree with evaluation about relative piece worth in exchange sequences.

```rust
// Duplicated — not linked to Eval
fn see_piece_value(p: Option<Piece>) -> i32 {
    match p {
        Some(Piece::Pawn) => 100,
        Some(Piece::Knight) => 320,
        // ...
    }
}

// In Eval impl — the real values
fn material_value(&self, piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => self.pawn_value,   // could be overridden
        Piece::Knight => self.knight_value,
        // ...
    }
}
```

## What to change

1. Delete `see_piece_value()`.
2. Change `see()`, `see_rec()`, `smallest_attacker()` to take `&Eval` and use `eval.material_value()`.
3. Update call sites — search already has access to Eval.

## Key interfaces

- `pub fn see(board, mv)` → `pub fn see(board, mv, eval: &Eval)` — or make it an Eval method
- `see_rec()` — already private, stays private, gains `eval` parameter
- `smallest_attacker()` — gains `eval` parameter for piece value comparison
- Search call sites: `eval::see(board, mv)` → `eval::see(board, mv, eval)` or `eval.see(board, mv)`

Prefer making `see()` a method on `Eval`: `Eval::see(&self, board, mv) -> i32`. This is cleaner — the caller already has `&Eval`.

## Acceptance criteria

- [ ] `see_piece_value()` deleted
- [ ] `see()` reads piece values from `Eval` — no hardcoded constants
- [ ] All SEE tests pass (5 existing tests)
- [ ] All 86 unit tests pass
- [ ] All 10 tactical integration tests pass
- [ ] No NPS regression (SEE is called in move ordering — performance neutral)
- [ ] Tuning `Eval::knight_value` affects SEE without code changes

## Out of scope

- Changing the SEE algorithm (recursive, smallest-attacker-first)
- Adding SEE to non-capture moves
- Making SEE configurable per-piece-type

## Comments
