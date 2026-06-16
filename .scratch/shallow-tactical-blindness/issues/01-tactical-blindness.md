# 01 — Material-loss blindness on quiet tactical blows

**Category:** bug
**Status:** `completed`

## Observed behavior

Two positions where the engine walks into decisive, single-move material loss:

### Case 1 — Pawn fork

**FEN:** `r3k2r/pp2qpp1/2n1b2p/2Ppp3/8/2PBP3/P1P2PPP/1R2QKNR w kq - 2 13`

Engine plays **Nf3 (g1f3)**. Black replies **e4**, a quiet pawn push forking the knight on f3 and the bishop on e3. White must lose a piece (~300 cp).

### Case 2 — Queen trap

**FEN:** `rnb1k2r/pp2pp1p/3p1np1/3P4/2B1P3/2qQ1N2/P1P2PPP/R1B2RK1 b kq - 1 10`

Engine plays **Qxa1 (c3a1)**, capturing the rook (+400 cp). White replies **Ba3**, a quiet bishop move trapping the queen on a1. The queen is lost (~500 cp net loss after the rook was already captured).

## Why this is surprising

The engine already has:

1. **Material evaluation** — piece values are central (`Eval::material_value`, pawn=100, knight=320, bishop=330, rook=500, queen=900). The search should see the material swing given enough depth.

2. **Mobility evaluation** (`src/eval/mobility.rs`) — scores each piece based on how many squares it can reach. A trapped piece should get a low mobility score.

3. **Quiescence search** (`src/search/quiescence.rs`) — extends the search on captures, promotions, and checks to resolve hanging material before evaluating.

The question is: **why do these existing mechanisms fail to catch these two obvious blunders?**

## Investigation areas

### 1. Why doesn't the search see deep enough?

Both blunders require 3+ ply to see the material loss:

- Case 1: Nf3 → e4 (quiet) → White move → QS captures hanging piece
- Case 2: Qxa1 (capture) → Ba3 (quiet) → Black move → ...eventual queen capture

At depth ≤ 2 from root, the quiet intermediate move (e4 / Ba3) falls into QS territory. Since QS only looks at captures/checks, the threat is invisible.

- What depth is the search reaching when it plays these moves?
- Is time control configuring too-shallow a search?
- Are pruning heuristics (null-move, futility, LMR) prematurely cutting off the lines that would reveal the threat?

### 2. Why doesn't mobility eval penalize these positions enough?

After the quiet threat move lands, the attacked piece's mobility should drop. However:

- `eval/mobility.rs:15`: mobility counts squares not occupied by *friendly* pieces (`attacks & !us_bb`), but does **not** exclude squares attacked by the enemy. A piece surrounded by enemy attacks can still show "high mobility."
- The mobility penalty floor is only **-20 cp** (at 0 safe squares) for all piece types (`params.rs:189-195`). A queen that lost all mobility is penalized just -20 cp — negligible against any material imbalance.
- In Case 2 after Ba3, the queen on a1 still has several reachable squares (a4, a5, etc.) so mobility barely drops.

### 3. Why doesn't static eval reflect undefended attacked pieces?

After e4, the knight on f3 and bishop on e3 are **both attacked** by the e4 pawn with **no White defender** of those squares. The static eval has no term that scores "piece X is attacked by a lower-value piece and has no defenders."

### 4. Could SEE help?

Static Exchange Evaluation (`src/eval/see.rs`) evaluates the net material outcome of a capture sequence. It is currently used only for move ordering, not for positional evaluation at leaf nodes. Perhaps SEE should be integrated into the static eval to discount positions where a piece is hanging.

## Acceptance criteria

- [ ] Identify the root cause (or causes) for both blunders
- [ ] Engine no longer plays Nf3 in Case 1 at `go depth 6`
- [ ] Engine no longer plays Qxa1 in Case 2 at `go depth 6`
- [ ] `cargo test --lib --test benchmarks` passes cleanly
- [ ] No significant NPS regression in `cargo test --release --test benchmarks -- --ignored`

## Out of scope

- Adding novel evaluation categories (threat detection, fork detection) — fix existing mechanisms first
- Full threat-move generation in QS — only consider if existing mechanisms are proven insufficient

## Comments

### 2025-06-16 — Focus on material loss, not mobility

Mobility is a much more difficult concept to evaluate — it's inherently positional, context-dependent, and calibrating it to catch tactics would distort its primary role. The imminent material loss at shallow depth is the elephant in the room and overwhelmingly the more likely root cause. The investigation should prioritize why the search + QS pipeline fails to see a 300-500 cp material swing that is only 2-3 plies away, rather than trying to make mobility double as a tactical detector.
