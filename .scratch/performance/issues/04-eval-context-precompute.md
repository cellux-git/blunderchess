# 04 — Pre-compute split piece bitboards in evaluate()

**Status**: `completed`
**Category**: `enhancement`

## Problem

`evaluate_side()` delegates to ~20 sub-evaluators, each of which independently calls `board.pieces_bb(Piece::X)` and `board.colors_bb(Color::Y)`. Across both sides, this is ~40 redundant bitboard lookups per `evaluate()` call. At 270K NPS, that's ~11 million redundant lookups per second.

## Solution

Add an `EvalContext` struct that pre-computes all 12 piece-color bitboards once per `evaluate()` call:

```rust
struct EvalContext {
    white_pawns: Bitboard,
    white_knights: Bitboard,
    white_bishops: Bitboard,
    white_rooks: Bitboard,
    white_queens: Bitboard,
    white_kings: Bitboard,
    black_pawns: Bitboard,
    black_knights: Bitboard,
    black_bishops: Bitboard,
    black_rooks: Bitboard,
    black_queens: Bitboard,
    black_kings: Bitboard,
    occ: Bitboard,
}
```

Construct once in `evaluate()` from `board` and pass to `evaluate_side()`. Sub-evaluators receive context directly instead of querying `board` repeatedly.

Each field is a single u64 (8 bytes) — total struct = 13 × 8 = 104 bytes, fits in two cache lines.

## Acceptance criteria

- [ ] `EvalContext` constructed once per `evaluate()` call
- [ ] All sub-evaluators accept `&EvalContext` instead of querying `board` repeatedly
- [ ] At least 30 `board.pieces_bb()` / `board.colors_bb()` calls removed from hot path
- [ ] All 86 unit tests pass
- [ ] All 10 tactical tests pass
- [ ] No functional eval change (same result)
- [ ] No NPS regression (expected improvement)

## Comments
