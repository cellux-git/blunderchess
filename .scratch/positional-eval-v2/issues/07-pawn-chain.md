# 07 — Pawn chain / phalanx bonus

**Status:** `completed`

**Status**: `completed`
**Category**: `enhancement`

## What's missing

Connected defended pawns (a "phalanx" — two pawns side by side on the same rank) get no bonus. A phalanx is a strong positional asset: the pawns control key squares, restrict enemy pieces, and are hard to attack.

Examples: white pawns on d4+e4, or c3+d3. These should get a small positional bonus.

Similarly, a pawn chain (diagonally connected, e.g. d4+c3 where d4 is defended by c3) is a classic strong structure. Currently only PST values reward this indirectly.

## What to add

- **Phalanx bonus**: two friendly pawns on the same rank and adjacent files
- **Pawn chain bonus**: pawn defended by a friendly pawn diagonally behind it on an adjacent file

```rust
pub pawn_phalanx_bonus: (i32, i32),
pub pawn_chain_bonus: (i32, i32),
```

## References

PRD gap #5 and #6.

## Comments
