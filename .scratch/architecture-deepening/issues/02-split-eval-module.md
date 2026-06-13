# 02 — Split eval.rs into sub-modules

**Status**: `completed`
**Category**: `enhancement`

## Problem

`eval.rs` is a 1,433-line monolith containing: the `Eval` struct (65 pub fields), 20 evaluation sub-methods, SEE logic, and bitboard geometry helpers. Test coverage is 11% — the lowest ratio in the codebase. Navigation is painful.

## What to change

Split into sub-modules under `src/eval/`, each with its own `#[cfg(test)] mod tests` section:

```
src/eval/
├── mod.rs            → orchestrator: evaluate(), evaluate_side(), game_phase(), material_value(), pst_value()
├── params.rs         → Eval struct definition + Default impl
├── pawns.rs          → eval_pawns, eval_pawn_chain, passed_pawns, eval_connected_passers, eval_candidate_passers, eval_passer_blocker
├── kings.rs          → eval_king_safety, eval_king_opposition, eval_king_passer_proximity
├── mobility.rs       → eval_mobility (knight, bishop, rook, queen MG+EG)
├── pieces.rs         → eval_rooks, eval_bad_bishops, eval_knights, eval_rook_queen_battery, eval_queen_multiattack, eval_exchange, eval_space, eval_pawn_majority
├── see.rs            → see, see_rec, attackers_to, smallest_attacker, see_piece_value
└── geometry.rs       → file_mask, adjacent_files_mask, rank_mask_forward, king_distance
```

Each sub-module uses `pub(crate)` visibility. `mod.rs` re-exports nothing new — the public API stays `Eval::evaluate()` and the `evaluate()` free function.

## Key interfaces

- `Eval` struct — unchanged public API
- `evaluate()` free function — unchanged
- `see()` free function — unchanged (callers use `eval::see()`)
- Internal seams: `pub(crate)` on sub-module items, accessible within the crate
- Tests move with their functions — zero test logic changes

## Acceptance criteria

- [ ] 8 sub-modules created as described
- [ ] `lib.rs` updated: `pub mod eval` → sub-modules are `pub(crate)` internally
- [ ] All 86 unit tests pass
- [ ] All 10 tactical integration tests pass
- [ ] No public API changes — crates consuming `blunderchess` see no difference
- [ ] Each sub-module has appropriate test coverage near its functions

## Out of scope

- Adding new tests beyond what currently exists
- Changing evaluation logic
- Eliminating DEFAULT_EVAL (issue #03)
- Unifying SEE piece values with Eval (issue #05)

## Comments
