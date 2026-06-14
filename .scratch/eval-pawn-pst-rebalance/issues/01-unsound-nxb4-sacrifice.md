# 01 — Engine plays unsound piece sacrifice (Nxb4) due to pawn PST bias

**Category:** bug
**Status:** ready-for-agent

## Summary

The engine plays 14..Nxb4 in the position below, sacrificing a knight for a pawn without adequate compensation.
The root cause is a pawn piece-square table (PST) bias of ~+358 cp favouring Black's "developed" pawn structure
over White's space-gaining pawn wedge, making all positions appear too good for Black.

**FEN:** `1rq1r2k/pbppbppp/1pn1pn2/8/1PPPP3/P4NP1/1BQN1PBP/2RR2K1 b - - 0 14`

## Current behavior

- Static eval of the initial (equal-material) position: **+457 cp for Black**
- PST + material breakdown: White PST+Mat MG = 24476, Black = 24834 → **+358 advantage for Black from PST alone**
- At depth 8 the engine selects `c6b4` (Nxb4) with score **+210**
- At depth 10 it alternates between Nxb4 and Bxb4 (the sensible bishop capture)
- At depth 12 it returns to Nxb4 with score **+142**
- The search PV: `c6b4 a3b4 e7b4 d4d5 e6d5 b2f6 d5e4 f6g7` — Black gives up two knights for four pawns,
  the engine sees approximately even material and evaluates the resulting position as favourable

## Desired behavior

- In the reported position (equal material), the static eval should be roughly neutral (within ±50 cp)
- Nxb4 should score **negative** (below −50 cp), reflecting the material deficit without true compensation
- The engine should prefer sensible developing moves (e.g. Bb7-a6, Be7-d6, h7-h6) over the sacrifice
- The fix should not break the existing test suite, especially:
  - `test_initial_position_symmetric` (initial position and 1.e4 must score within ±50 cp)
  - `test_initial_position_near_zero` (initial position must score within ±50 cp)
  - `test_white_advantage_positive` (being up material must improve score)
  - All 13 tactical integration tests in `tests/benchmarks.rs`

## Key interfaces

- `Eval::mg_pawn_table` — the MG pawn PST table in `src/eval/params.rs`. Row 1 (rank 2 from White's perspective,
  rank 7 from Black's) has values ranging 60–134; these heavily reward "one-step-developed" pawns. Black's pawns
  on rank 7 collectively get ~509 MG PST vs White's ~170, creating a +339 raw pawn-PST bias.
- `Eval::eg_pawn_table` — the EG pawn PST table contributes additional bias (178–187 for row 1).
- `Eval::space_bonus` — currently (5, 3) per advanced pawn. White's four advanced pawns (b4,c4,d4,e4) get only
  +20 MG from space. This is dwarfed by the PST bias.
- `Eval::evaluate()` and `Eval::evaluate_side()` — no code changes needed; only the table values in `Eval::default()`.

## Acceptance criteria

- [ ] Static eval of `1rq1r2k/pbppbppp/1pn1pn2/8/1PPPP3/P4NP1/1BQN1PBP/2RR2K1 b - -` is within ±80 cp (was +457)
- [ ] At depth 8, the engine does **not** play `c6b4` (Nxb4) — score for that move must be ≤ 0
- [ ] `test_initial_position_symmetric` passes (1.e4 eval within ±50 cp)
- [ ] `test_initial_position_near_zero` passes (startpos within ±50 cp)
- [ ] `test_white_advantage_positive` passes
- [ ] All 13 tactical tests in `tests/benchmarks.rs` pass
- [ ] `cargo test --lib --test benchmarks` passes cleanly

## Out of scope

- Tuning non-pawn PST tables (knight, bishop, rook, queen, king)
- Tuning mobility tables
- Tuning king safety parameters
- Tuning any positional bonus/penalty other than space_bonus
- Changing the search algorithm (alpha_beta, LMR, null-move, quiescence)
- Fixing other eval biases not directly related to the pawn PST causing this specific sacrifice

## Diagnostic data

From `Eval::default().evaluate()` on the reported FEN with the **unmodified** tables:

| Component            | White  | Black  | Diff (B−W) |
|----------------------|--------|--------|------------|
| PST+Material MG      | 24476  | 24834  | +358       |
| Mobility MG          | 90     | 76     | −14        |
| King safety MG       | −36    | −24    | +12        |
| Unexplained (pawns, bishops, rooks, space, etc.) | — | — | ~+101 |
| **Total**            |        |        | **+457**   |

Black's pawn PST dominance: MG values for Black's 8 pawns ≈ 509 vs White's ≈ 170.
The main contributors are the rank-1 (from each side's perspective) pawn PST entries in `mg_pawn_table` row 1
(currently 98, 134, 61, 95, 68, 126, 34, −11) and the under-rewarded advanced-pawn ranks (rows 3–4).

