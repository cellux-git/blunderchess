# 09 — Passed pawn blocker bonus

**Status:** `completed`

**Status**: `completed`
**Category**: `enhancement`

## What's missing

A piece (especially knight, bishop, or king) that sits directly in front of an enemy passed pawn and blocks its advance should get a bonus. The engine already has `rook_behind_passer` but no generic "piece blocking enemy passer."

A knight on d3 blocking a black passed pawn on d4 is a classic defensive resource. The blocker neutralizes the passer and should be rewarded.

## What to add

For each enemy passed pawn, check if a friendly piece occupies the square directly in front of it (the blockade square). Apply a bonus per blocker, scaled by piece value (knight gets more than king, which is already there for defense).

```rust
pub passer_blocker_bonus: (i32, i32), // mg, eg — base bonus
pub passer_blocker_noble_multiplier: i32, // multiplier for knight/bishop (e.g. 2x vs king)
```

King blocking own passer: already handled by `king_passer_proximity`. King blocking enemy passer: this is the gap.

## References

PRD gap #8.

## Comments
