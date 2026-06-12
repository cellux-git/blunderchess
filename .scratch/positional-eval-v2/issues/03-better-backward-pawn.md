# 03 — Better backward pawn detection

**Status**: `ready-for-agent`

## Current state

`eval_pawns` checks backward pawns by:
1. Pawn has no friendly pawn behind it on adjacent files
2. Enemy pawn is behind on adjacent files

This misses the key requirement: a backward pawn is one whose forward square (one step ahead) is controlled by at least one enemy pawn, making it unsafe to advance. Example: white pawn on d3, black pawns on e5 and c5 controlling d4 → white pawn on d3 is backward.

## What to change

1. Modify the backward pawn check in `eval_pawns`:
   - Compute the square one step forward from the pawn
   - Check if ANY enemy pawn attacks that square (use `pawn_attacks(forward_sq, color)` intersected with enemy pawns)
   - If the forward square is attacked by at least one enemy pawn AND the pawn is unsupported by friendly pawns on adjacent files behind it → it's backward
   - Also: if the forward square has a friendly pawn, the pawn is NOT backward (it's part of a pawn chain)

2. The check should be: pawn can't advance because the square in front is attacked by at least one enemy pawn, AND no friendly pawn on adjacent file behind it can recapture if it pushes.

3. Add a test: white pawn on d3, black pawns on c5 and e5 (controlling d4) → backward pawn penalty applied to d3.

## Acceptance criteria

- Backward pawns detected when forward square is attacked by enemy pawn
- Pawns in a chain or with friendly support are NOT flagged as backward
- No regression on existing pawn structure tests

## Comments
