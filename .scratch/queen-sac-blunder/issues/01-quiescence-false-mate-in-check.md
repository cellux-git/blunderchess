# 01 — Quiescence false mate detection when king is in check

**Status:** `completed`

**Category:** `bug`

## FEN

```
rnb3k1/pp3pb1/3p2pp/2pP4/4Nq2/4P3/PP3PBP/R2Q1RK1 b - - 0 16
```

## Symptom

Engine plays `f4h2` (Qxh2??), sacrificing the queen for a pawn. The queen on f4 is attacked by the pawn on e3 but is not trapped — it has many safe squares (h4, g5, e5, etc.).

## Root cause

In `src/search/quiescence.rs`, when the side to move is in check, the move filter at `qs_depth > 0` only considered captures (`include = is_cap_or_promo || qs_depth == 0`). When the only way to escape check required a quiet move (e.g. Kh1, Kg1 after Bg7-e5+), no moves passed the filter, `filtered == 0`, and quiescence returned `-(CHECKMATE - ply)` — a false mate score.

The specific sequence:
1. Black plays Qxh2+ (ply 0, qs_depth=0) — included because qs_depth=0
2. White recaptures Kxh2 (ply 1, qs_depth=1) — included because it's a capture
3. At ply 2, Black's Bg7-e5+ gives check to White king on h2 — included because it gives check and `in_check` was false at ply 2 (`gives_check` passes the filter)
4. At ply 3, White is in check from e5 bishop. White can escape with Kh1 or Kg1, but these are quiet moves and at qs_depth=1 they fail the `include` check. Captures (Nxc5, Nxd6) don't escape check. Result: `filtered == 0`, `in_check == true`, returns `-(CHECKMATE - 3)` = -999,997 for White, which negates to +999,997 for Black — a false mate-in-2 score.

## Fix

Changed the `include` condition in quiescence from:

```rust
let include = is_cap_or_promo || qs_depth == 0;
```

to:

```rust
let include = is_cap_or_promo || qs_depth == 0 || in_check;
```

When the side to move is in check, all legal moves are now considered at any qs_depth, because escaping check takes priority over the capture-only restriction. The downstream filter (`own_king_safe && (is_cap_or_promo || gives_check || in_check)`) already correctly passes moves that get out of check.

## Files changed

- `src/search/quiescence.rs:33` — one-line fix

## Test

`test_queen_not_sacrificed_when_attacked` in `src/search/tests.rs` — searches the position at depth 4 and asserts the best move is not `f4h2`.
