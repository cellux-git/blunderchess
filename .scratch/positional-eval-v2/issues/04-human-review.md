# 04 — Human review: revise evaluations and suggest new ones

**Status:** `completed`


**Status:** `completed`

## Context

The engine now has a functional positional evaluation with 14+ terms (pawn structure, mobility, king safety, outpost knights, passer bonuses, bishop pair, rook files, etc.) and an `Eval` struct with 40+ tunable fields.

Most of these were implemented by an agent that does not play chess. The user knows chess and should review the evaluation from a player's perspective.

## What to review

1. **Weight sanity**: Do the default weights match chess intuition?
   - Bishop pair bonus (25 mg, 45 eg) — too high/low?
   - Knight outpost (18 mg, 8 eg) — reasonable?
   - Connected passers (20 mg, 40 eg) — enough?
   - Trapped bishop penalty (-40 mg, -50 eg) — too harsh?
   - King shield (-12 per missing pawn) — calibrated right?
   - Mobility tables — do the values per square count make sense?

2. **Missing terms**: What positional concepts does the engine still miss that a human player would consider important?
   - Piece coordination (rook battery, queen+bishop alignment)
   - Color complexes (weak squares of a given color)
   - 7th rank rook (rook on the opponent's 2nd rank)
   - Pawn breaks / lever evaluation
   - Hanging pieces
   - Development advantage in opening
   - Initiative / tempo
   - Zugzwang detection in pawn endgames
   - King safety in queenless middlegames
   - Piece activity vs material balance tradeoffs

3. **Redundancy / overlap**: Are any terms double-counting the same concept?
   - Mobility vs PST (central squares get both PST bonus and mobility bonus)
   - King shield vs pawn structure (shield missing = isolated/doubled pawns?)
   - Rook on open file vs passed pawns (rook behind passer already rewarded)

4. **Phase blending**: Are the MG/EG transitions smooth? Do any terms jump discontinuously?
   - Mobility is MG-only (can't jump from full bonus to zero at phase=0)
   - King safety is MG-only (does a castled king instantly become safe in endgame?)

## Deliverable

After reviewing, the user should:
- Add comments to this issue with observations
- Create new issues under `.scratch/positional-eval-v2/issues/` for concrete improvements
- Adjust default weights if desired (edit `Eval::default()` in `src/eval.rs`)

## Comments
