# 08 — Candidate passer detection

**Status**: `completed`
**Category**: `enhancement`

## What's missing

A "candidate passer" is a pawn that can become a passed pawn by capturing an enemy pawn on an adjacent file. Currently only actual passed pawns (no enemy pawns ahead) get the passed pawn bonus.

Example: white pawn on d4, black pawn on e5. If white can play dxe5, the d-pawn becomes passed. The engine should recognize this potential and give a smaller bonus.

## What to add

For each pawn, check if there's exactly one enemy pawn on an adjacent file ahead of it. If capturing that pawn would make it passed, apply a candidate passer bonus (e.g. 50% of the full passer bonus for that rank).

```rust
pub candidate_passer_bonus: [i32; 8], // per rank, subset of passed_pawn_bonus
```

## References

PRD gap #7.

## Comments
