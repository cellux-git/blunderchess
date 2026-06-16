# 12 — Pawn majority on a wing

**Status:** `completed`

**Status**: `completed`
**Category**: `enhancement`

## What's missing

When one side has more pawns on a flank (queenside: files a-d, or kingside: files e-h), advancing those pawns to create a passed pawn is a key endgame plan. The engine doesn't evaluate pawn majorities.

Example: white has pawns on a2, b2, c2 vs black's a7, b7 — white has a queenside majority. With correct play, white can create a passed pawn on the queenside. The engine should recognize this as a long-term advantage.

## What to add

Count pawns on each wing (queenside a-d, kingside e-h) for both sides. If one side has more pawns on a wing, apply a bonus. The bonus should scale with how advanced the majority is (pawns on the 4th/5th rank are closer to creating a passer).

```rust
pub pawn_majority_bonus: (i32, i32), // mg, eg — per extra pawn in majority
pub pawn_majority_advance_bonus: (i32, i32), // extra per advanced pawn in majority
```

Start simple: just count the imbalance on each wing and apply a small bonus. Advanced versions can penalize crippled majorities (where the majority is blocked).

## References

PRD gap #12.

## Comments
