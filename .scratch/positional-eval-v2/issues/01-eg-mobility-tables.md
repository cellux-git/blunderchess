# 01 — Add endgame mobility tables

**Status**: `ready-for-agent`

## Current state

Mobility evaluation (`eval_mobility`) computes safe squares for knights, bishops, rooks, and queens. The result is added to `mg_score` only — it contributes zero to `eg_score`. In endgames, piece mobility is completely ignored.

## What to change

1. Add EG mobility table fields to `Eval`:
   ```rust
   pub knight_mobility_eg: [i32; 9],
   pub bishop_mobility_eg: [i32; 14],
   pub rook_mobility_eg: [i32; 15],
   pub queen_mobility_eg: [i32; 28],
   ```

2. Modify `eval_mobility` to return `(i32, i32)` — MG and EG components:
   ```rust
   fn eval_mobility(&self, board: &Board, color: Color, enemy: Color, occ: u64) -> (i32, i32)
   ```
   Compute both mg and eg scores using the respective tables.

3. In `evaluate_side`, add both to mg_score and eg_score respectively (neither gets internal phase weighting — the outer blend in `evaluate()` handles it).

4. Provide sensible endgame defaults: EG mobility values should be lower than MG (mobility matters less in endgames), but still non-zero. Start with 50% of MG values and tune from there.

5. Add a test: position with two knights (one centralized, one in corner) in an endgame (phase=0) should score higher for the centralized knight.

## Acceptance criteria

- `eval_mobility` returns `(mg, eg)` instead of `i32`
- Both `mg_score` and `eg_score` in `evaluate_side` receive mobility contributions
- At phase=0 (pure endgame), centralized pieces still get a mobility bonus
- No NPS regression (>250K at depth 6 in release)
- All existing tests pass

## Comments
