# Evaluation design: principles, not specifics

**Status**: accepted

## Context

The evaluation function sits on the hot path — called millions of times per search. Each additional term adds cost and complexity. There is a natural temptation to add specific rules for known tactical or positional patterns (e.g. "penalize a knight on e4 when white has pawns d4+e5"), but this leads to an unmaintainable eval: many narrow rules that rarely fire, interact unpredictably, and are hard to tune.

## Decision

The evaluation shall consist of a **handful of generic, principled terms** that capture positional concepts broadly. Specific bad positions should be caught by the interaction of these principles, not by dedicated one-off rules.

### Canonical eval terms (principle-based)

| Term | Principle it captures |
|------|----------------------|
| Material + PST | Piece value + centralization, development |
| Mobility (safe squares) | Piece activity, scope, freedom |
| King safety | Vulnerability of the king |
| Pawn structure | Long-term positional skeleton |
| Bishop pair | Synergy of the two-bishop advantage |
| Outpost | Pieces anchored on squares the opponent can't challenge |
| Rook files (open/semi-open) | Rook activity on open lines |
| Passed pawns | Advancing potential of free pawns |
| Space | Territory control in the opponent's half |
| Exchange evaluation | Rook-vs-minor-piece positional nuances |

### Anti-pattern: specific rules

A specific rule would be: "when Black has a knight on e4, White pawns d4+e5, and Black plays Bb4+, penalize the knight." This catches exactly one position family and nothing else. Rejected.

Instead, the position should be caught by:
- **Mobility**: the knight's safe squares are limited (enemy pawn attacks are excluded) → low mobility score
- **Material/defense**: an undefended piece in the enemy half is vulnerable → no outpost bonus (outpost requires pawn defense)
- **King safety**: any threat to the king from the check is resolved by the search seeing the c3 block

### Refining principles, not adding rules

When the eval fails to recognize a bad position, the response should be:

1. Identify which **principle** should have caught it
2. Tighten that principle's implementation
3. Only add a new term if an entirely new principle is needed

Example from the Bb4+ trap (`.scratch/bb4-knight-trap/issues/01-bb4-knight-trap.md`):
- The engine played Bb4+ because it incorrectly credited the knight on e4 with an outpost bonus and full mobility
- Fixed by tightening the **outpost** principle (require actual pawn defense) and the **mobility** principle (exclude squares attacked by enemy pawns)
- No new evaluation term was added

Example from the f7f6 passive pawn push:
- The engine played f7f6 over Bg7 because the mg_pawn_table gave a +51 cp bonus for pushing f7→f6 (row 2 values up to 65), vs only +5 cp for developing Bf8→g7
- The PST **is** the development principle — it encodes piece centralization, pawn advancement, and king castling position
- Fixed in two layers:
  1. Moderated mg_pawn_table row 2 (rank 3/6) from `30,7,26,50,65,56,60,-20` to `10,10,18,25,28,22,20,-5` — pawn advancement still rewarded but no longer overwhelms piece development
  2. Deepened mg_bishop_table back-rank penalties (c1/f1: 0→-15, c8/f8: -10→-15) and increased early-development rewards (g7/b7: -5→5, g2/b2 similarly) — undeveloped pieces penalized, developing a bishop now gains +20 cp instead of +5 cp
- Net result: f7→f6 PST gain reduced from 51→17 cp, Bf8→Bg7 gain increased from 5→20 cp. Static eval now prefers Bg7 by 23 cp.
- The king PST already encodes castling benefit (g8=+10 vs e8=-10, +20 cp for castling); the engine finds this through search depth
- No new "development" term was added — the principle was already present, just poorly calibrated

## Why

- **Maintainability**: fewer terms → fewer interactions → easier tuning
- **Speed**: every term has a CPU cost. Principle-based terms fire often enough to justify their cost
- **Generality**: a principled term like "mobility" evaluates knight position everywhere, not just on e4
- **Tunability**: principle-based terms have smooth, continuous scoring that responds gracefully to small positional changes

## Consequences

- New eval terms are added sparingly and only when they represent a distinct positional concept
- Bug reports about bad eval are investigated by asking "which principle should have caught this?"
- The canonical list of eval terms in ADR-0005 serves as the checklist of principles
- Each principle's implementation must be correct in isolation — a single loose check (like the old outpost "pawn on adjacent file") undermines the whole approach
