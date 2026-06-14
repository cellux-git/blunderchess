# Eval parameter grouping into six domain sub-structs

**Status**: accepted

## Context

The `Eval` struct in `src/eval/params.rs` holds 59 `pub(crate)` fields — piece values, 12 PST tables, 8 mobility tables, and ~25 positional eval parameters. Every eval term accesses the full struct via `&self`, even when it only reads 2-3 fields. Tests for individual eval terms must either construct a full `Eval` (with all 59 fields) and zero out unrelated fields, or test through the full `evaluate()` pipeline and infer correctness from aggregate scores.

This makes eval tuning brittle: changing a pawn-structure penalty can break a mobility test because the test can't isolate the term under test.

## Decision

Group the 59 fields into six sub-structs, each owned by `Eval`:

| Sub-struct | Fields | Used by |
|------------|--------|---------|
| **MaterialValues** | 6 piece material values | evaluate_side (inline material+PST loop) |
| **PieceSquareTables** | 12 PST tables (mg+eg × 6 pieces) | evaluate_side (inline material+PST loop) |
| **MobilityTables** | 8 mobility tables (mg+eg × N,B,R,Q) | eval_mobility |
| **PawnEval** | doubled, isolated, passed, backward, phalanx, chain, candidate, blocker, space, majority | pawns.rs, pieces.rs (space/majority) |
| **PieceEval** | bishop pair, bad bishop, rook files/7th/battery, knights outpost/rim/trapped, queen attacks/fork, exchange | pieces.rs |
| **KingEval** | king shield/open file, opposition, king-passer proximity, connected passer, rook-behind-passer | kings.rs, pawns.rs (connected/rook-behind) |

Each `eval_*` function takes its sub-struct as `&self` (or as a parameter) rather than the full `Eval`. Tests construct only the sub-struct under test. The `Eval` struct becomes a thin container holding the six sub-structs and the `Default` impl delegates to each sub-struct's default.

## Why

- **Test isolation**: a pawn-structure test constructs `PawnEval::default()` and calls `eval_pawns()` directly. Unrelated fields (mobility tables, piece values) don't exist in the test's scope.
- **Locality**: changing `passed_pawn_bonus` requires touching only `PawnEval`, not the 59-field `Eval`.
- **Leverage**: each sub-struct interface gates 5-10 eval terms behind a small set of fields.
- **Deletion test**: delete `PawnEval` — all pawn-structure parameters and logic concentrate in one place. Callers that need pawn eval must depend on `PawnEval`, not the full `Eval`.

## Considered options

| Option | Rejected because |
|--------|------------------|
| 59 flat fields (current) | No test isolation. Interface = implementation. Every eval tuning task fights the flat struct. |
| Four broad groups (Material+PST, Mobility, Positional, PassedPawn) | "Positional" is a grab-bag. King-safety and rook-eval are conceptually distinct enough to justify separate modules. |
| One sub-struct per eval term (~20 structs) | Too many seams. Many terms share parameters (e.g. rook open/closed/7th all use the same rook file masks). Grouping by domain reduces boilerplate. |

## Consequences

- `src/eval/params.rs` shrinks to ~30 lines (six struct declarations + `Eval` container).
- Each sub-struct gets a separate source file or a subsection in `params.rs`.
- `evaluate_side` signature does not change — it still takes `&self` (the `Eval` container). Internally it dispatches to `self.pawn_eval.eval_pawns(...)`.
- Existing tests that construct `Eval::default()` continue to work — `Eval::default()` still returns a fully-populated struct.
- Future eval terms added under the appropriate sub-struct without expanding a flat list.
