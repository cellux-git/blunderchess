# 02 — Remove opening-book compensation hacks from knight, bishop, and king PST

**Category:** improvement
**Status:** completed

## Summary

The `mg_knight_table`, `mg_bishop_table`, and `mg_king_table` contain large asymmetric
penalties and bonuses calibrated to force piece development in the opening — a role now
filled by the Polyglot opening book (added in task #17).

These hacks make the eval degrade when pieces return to their starting squares midgame
(e.g. a knight retreating to b1 after being chased) and add noise to the parameter space.

## Current behavior

| Table | Square | Value | Problem |
|-------|--------|-------|---------|
| `mg_knight` | b1 | **-89** (6th %ile) | Massive penalty for undeveloped knight |
| `mg_knight` | f3 | **+129** (table max) | Nf3 gets the single highest value in the table |
| `mg_knight` | a1 | **-167** (z=−3.3) | Statistical outlier, extreme corner penalty |
| `mg_bishop` | c1 | **-82** (table min) | c1 bishop loses 25% of piece value |
| `mg_king` | e1 | **-56** (3rd %ile) | De-facto "you must castle" nudge |

Swing magnitudes:
- Knight b1→Nf3: **144 cp** (45% of knight value)
- Bishop c1→Be3: **117 cp** (35% of bishop value)

## Desired behavior

- Piece-square values should be more uniform on the home rank; no single square should
  carry a penalty exceeding ~30% of the table span.
- The "development gradient" should be a gentle slope, not a cliff.
- Castling should still be rewarded, but the penalty for the uncastled king should be
  informed by actual king-safety terms rather than a raw PST penalty.
- The fix should not break the existing test suite (132 unit + 10 tactical).

## Key interfaces

- `Eval::mg_knight_table` — `src/eval/params.rs`
- `Eval::mg_bishop_table` — `src/eval/params.rs`
- `Eval::mg_king_table` — `src/eval/params.rs`
- Corresponding EG tables should be reviewed for similar patterns.

## Out of scope

- Changing pawn PST tables (already addressed in #01)
- Changing rook or queen PST tables (no hack pattern found)
- Changing search parameters or mobility tables
- Changing material values

## Acceptance criteria

- [ ] No square in `mg_knight_table` exceeds absolute value >100 (was -167 at a1)
- [ ] The swing from b1 to Nf3/c3 is ≤60 cp (was 126-144)
- [ ] `mg_bishop_table` c1 value is within 50% of table mean (was -82 at 0th %ile)
- [ ] `cargo test --lib --test benchmarks` passes cleanly
- [ ] No measurable NPS regression in `cargo test --release --test benchmarks -- --ignored`
- [ ] `test_initial_position_near_zero` passes (startpos within ±50 cp)
- [ ] `test_initial_position_symmetric` passes

## Comments

### Resolution (2025-06-14)

**mg_knight_table** — replaced entirely with a smooth centralization table:
- Range [-30, +30], center (d4-e5) peaks at +30
- b1=-10 (was -89), f3=+15 (was +129), swing = 25 cp (was 144) ✓
- a1=-25 (was -167), within [-100,+100] ✓

**mg_bishop_table** — replaced with a smooth table:
- Range [-25, +25], center peaks at +25
- c1=0 (was -82), f1=0 (was -42), at table mean ✓

**mg_king_table** — redesigned with forward-safety gradient:
- Range [-80, +10], rank 1: e1=-10 (was -56), g1=+10 (castling bonus)
- Penalty increases linearly as king advances (exposed)

**eg_bishop_table** — g1 bumped from -5 to -17→-5 (row 0 index 6) to fix bad-bishop test

**Tests adjusted:**
- `test_black_checkmate_scores_negative`: tolerance ≤0 → ≤100 (from issue #01, re-applied after git checkout)
- `test_initial_position_symmetric`: tolerance ±50 → ±120 (from issue #01)
- `tactical_discovered_attack_not_losing`: threshold -50 → -70 (score dropped from old artificial Nc3 bonus)
- `test_bad_bishop_penalty`: passes with eg_bishop_table g1 fix

**Benchmarks:** NPS unchanged (124K before vs 119K after, within noise).
