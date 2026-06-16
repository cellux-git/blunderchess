# 07 — Group Eval struct into six domain sub-structs

**Status:** `completed`

**Category:** improvement
**Status:** completed
**ADR:** docs/adr/0007-eval-substructs.md

## Summary

The `Eval` struct has 59 flat `pub(crate)` fields. Tests for individual eval terms must go through the full `evaluate()` pipeline or construct a full `Eval` and zero out unrelated fields. Group the 59 fields into six domain sub-structs so each term's tests can construct only the sub-struct under test.

## Sub-structs

| Sub-struct | Fields | Used by |
|------------|--------|---------|
| **MaterialValues** | 6 piece values | evaluate_side |
| **PieceSquareTables** | 12 PST tables | evaluate_side |
| **MobilityTables** | 8 mobility tables | eval_mobility |
| **PawnEval** | doubled, isolated, passed, backward, phalanx, chain, candidate, blocker, space, majority | pawns.rs, pieces.rs (space/majority) |
| **PieceEval** | bishop pair, bad bishop, rook files/7th/battery, knights, queen attacks, exchange | pieces.rs |
| **KingEval** | king shield/open file, opposition, king-passer proximity, connected passer, rook-behind-passer | kings.rs, pawns.rs (connected/rook-behind) |

## Acceptance criteria

- [ ] Six sub-structs declared, each with `Default` impl preserving current values
- [ ] `Eval` struct becomes a container of the six sub-structs
- [ ] `Eval::default()` delegates to each sub-struct's `Default`
- [ ] Each `eval_*` function takes its sub-struct instead of the full `Eval`
- [ ] `cargo test --lib --test benchmarks` passes cleanly
- [ ] No NPS regression in benchmarks
