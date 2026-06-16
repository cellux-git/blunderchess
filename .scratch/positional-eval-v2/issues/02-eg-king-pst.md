# 02 — Endgame king PST (centralization)

**Status:** `completed`

**Status**: `completed`

## Current state

The king PST table (`mg_king_table` / `eg_king_table`) is tuned for castled kings — it rewards kings on g1/c1 (White) and g8/c8 (Black). In endgames (phase→0), the engine still evaluates kings by this castling-centric table.

A king on e4 in the endgame should be worth more than a king on g1, but the current EG table doesn't strongly differentiate.

## What to change

1. Replace `eg_king_table` with a table that peaks in the center and decays toward the edges:
   - Center squares (d4/e4/d5/e5): +20 to +25
   - Extended center (c3-c6, f3-f6): +10 to +15
   - Edges: 0 or slightly negative
   - Corners: -20

2. The MG table should remain castling-friendly (keep current PeSTO values).

3. Update `Eval::default()` with the new table. The table values are symmetric (White's view, flipped for Black via `sq.index() ^ 56`).

4. Add a test: in an endgame position (only kings + 1-2 pawns, phase=0), a centralized king scores higher than a cornered king.

## Acceptance criteria

- New `eg_king_table` rewards centralization, penalizes corners
- Test: KPPvK endgame, king on e4 vs g1 → centralized scores higher
- All existing tests pass

## Comments
