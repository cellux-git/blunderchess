# Positional Evaluation v2

**Status:** `completed`


**Status:** `completed`

## Goal

Strengthen the static evaluation function beyond the current PeSTO PSTs + basic pawn structure + basic mobility. The engine currently evaluates at ~270K NPS with correct but tactically-biased scoring. Several known gaps prevent it from understanding long-term positional themes.

## What we have (v1)

- PeSTO piece-square tables (MG/EG) with material phase interpolation
- Pawn structure: doubled (per-back-pawn), isolated, passed, backward
- Bishop pair bonus, trapped bishop (corner patterns only)
- Rook open/semi-open file bonuses
- Piece mobility for N/B/R/Q (MG only, linear tables)
- King safety: pawn shield, open files near king, enemy pieces in king zone
- Outpost knights, connected passers, rook behind passer, king-passer proximity

## Known gaps

1. **Mobility is MG-only**: there are no EG mobility tables. Knights and bishops that are strong in the endgame (centralized, supporting passed pawns) get no mobility bonus after phase 0.
2. **King safety is MG-only and simplistic**: no pawn storm detection, no queen proximity, no semi-open file attacks on castled king.
3. **No piece-square table for the king in EG**: the PeSTO king table is tuned for castled kings. In endgames, kings should centralize but the table doesn't reward it strongly enough.
4. **Backward pawn detection is weak**: only checks enemy pawns behind on adjacent files. A proper backward pawn is one that can't safely advance because the square in front is controlled by at least one enemy pawn.
5. **No pawn chain evaluation**: connected defended pawns (phalanx) have no bonus.
6. **Trapped bishop is corner-only**: doesn't detect bishops trapped behind own pawns on same-color squares in the center.
7. **No candidate passer detection**: a pawn that can become passed by capturing an enemy pawn is not recognized.
8. **No passed pawn blocking**: a piece (especially knight or king) blocking an enemy passer should get a bonus.
9. **No king opposition in pawn endgames**: K+P vs K requires exact distance calculations.
10. **No fortress detection**: K+Q vs K+R endgames, etc.
11. **No space advantage**: control of the center 4 squares (d4/e4/d5/e5) is not rewarded.
12. **No pawn majority on a wing**: advancing pawns on the side where you have more pawns is a key endgame plan.
13. **No evaluation of the exchange**: the trade of rook for bishop/knight (the "exchange") is just material difference; positional factors (open files, pawn structure, king safety) heavily affect whether the exchange is good.
14. **Mobility tables are linear**: real mobility follows a logarithmic curve (first few squares matter most).
15. **No king-passer proximity in MG**: only EG. In queenless middlegames, king activity matters.

## Implementation approach

Each gap should be implemented as a separate, independently testable feature with:
- A new field on `Eval` with a sensible default weight
- A focused evaluation method in `Eval`
- At least one unit test verifying the feature works
- No performance regression in benchmarks

## Priority ordering

| # | Feature | Priority | Effort | Risk |
|---|---------|----------|--------|------|
| 1 | EG mobility tables | High | Low | Low |
| 2 | EG king PST (centralization) | High | Low | Low |
| 3 | Better backward pawn detection | Medium | Low | Low |
| 4 | Candidate passer detection | Medium | Low | Low |
| 5 | Passed pawn blocker bonus | Medium | Low | Low |
| 6 | Pawn chain (phalanx) bonus | Medium | Low | Low |
| 7 | King opposition (K+P endgames) | Medium | Medium | Medium |
| 8 | Space/center control | Low | Low | Low |
| 9 | General trapped bishop (beyond corners) | Low | Medium | Low |
| 10 | Pawn majority on a wing | Low | Low | Low |
| 11 | Exchange evaluation | Low | Medium | Medium |
| 12 | Passed pawn storm (MG king safety) | Low | Medium | Medium |
| 13 | Fortress detection | Low | High | High |

## Phase 2: Human review

After all agent-implemented evaluation features are complete, the user will conduct a manual review of the evaluation (see [issue 04](issues/04-human-review.md)). The user knows chess and will:

- Audit default weights for chess-intuitive correctness
- Identify missing positional concepts
- Check for redundant or overlapping terms
- Suggest new evaluation features based on actual chess knowledge

This review may produce additional issues beyond the initial 13.
