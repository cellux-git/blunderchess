# How BlunderChess thinks: a search algorithm walkthrough

This document explains the search algorithm in plain language. You don't need prior
chess programming knowledge — just basic familiarity with chess rules.

---

## The problem

A chess engine, given a position, must answer: *what is the best move?*

A human does this by imagining a few candidate moves, then thinking "if I play
this, my opponent could reply with that, then I could play this..." — a mental
tree of possibilities. An engine does the same thing, systematically and at scale.

The engine's tree has nodes (positions) and edges (moves). The root is the
current board. Children are positions after one move. Grandchildren are positions
after two moves. The engine explores this tree recursively.

---

## Step 1: Tapered material + positional evaluation

At the leaves of the tree, the engine judges "how good is this position?" with
a **tapered evaluation** that blends midgame and endgame scores by material phase
(Pawn=0, Knight/Bishop=1, Rook=2, Queen=4; max phase=24).

Components:
- **Material + piece-square tables (PeSTO-derived)**
- **Pawn structure**: doubled pawns (−12mg/−24eg), isolated pawns (−12/−20),
  passed pawn bonuses by rank (0..120), backward pawns (−8/−16), connected
  passers (+20)
- **King safety**: pawn shield (2-3 squares in front, −45 per missing pawn),
  open adjacent files (−30), attacker zone (8-zone around king, +15 per
  enemy piece)
- **Mobility**: knight (weighted by safe reachable squares, capped at 8),
  bishop (capped at 13), rook (capped at 14), queen (capped at 27)
- **Bishop pair**: +40mg/+60eg when side has both bishops
- **Trapped bishops**: penalty for bishop blocked by own pawns on a2/h2/a7/h7
- **Rook files**: open file +25mg/+18eg, semi-open +12mg/+8eg, rook behind
  own or enemy passer bonus
- **Outpost knights**: +15mg/+10eg for knights on 5th/6th rank protected by
  own pawn
- **King-passer proximity**: bonus for king distance to passers in endgame

Piece values, PSTs, and all weights live on a single `Eval` struct with
`OnceLock<Eval>` static default — per-call allocation eliminated. Custom values
settable via `Eval::evaluate()` for tuning.

---

## Step 2: Minimax → alpha-beta

Basic minimax: white picks the move with the highest score; black picks the
move with the lowest score. Full tree exploration explodes combinatorially.

**Alpha-beta pruning** tracks two bounds — alpha (best score for maximizing
side) and beta (best for minimizing side). If at any point alpha ≥ beta, we
stop searching that branch: the opponent has a better option elsewhere.
With good move ordering, alpha-beta reduces the search from ~35^n to ~35^(n/2).

---

## Step 3: Principal Variation Search (PVS)

A refinement: search the first move with a full window, then try all remaining
moves with a **null window** (score, score+1). If the null window fails high,
re-search with the full window. In practice, the first move is best >90% of the
time, saving enormous search effort.

BlunderChess also reduces depth for late quiet, non-killer moves via **Late Move
Reductions (LMR)**. Moves 4-7 get R=1, moves 5-7 get R=2, moves ≥8 get R=3.
Killer moves and checks are never reduced.

---

## Step 4: Iterative deepening + aspiration windows

Search depth 1, then 2, 3, 4... until time runs out or stop flag is set.
Each completed iteration provides a result immediately. Deeper iterations benefit
from shallower results via the transposition table.

**Aspiration windows** narrow the root search window at depth ≥ 4 (±25cp around
the previous depth's score). On fail-low or fail-high, widen and re-search.
Saves ~10% search time.

A stop flag (`AtomicBool`) is checked after each completed depth iteration.
When set, the search unwinds and returns the last completed depth's best move.

---

## Step 5: Transposition table

Different move sequences can reach the same position. The TT is a lock-free
hash table:

- **Key**: 64-bit Zobrist hash, updated incrementally as moves are made
- **Value**: (score, depth searched, node type, best move)
- **Node types**: Exact (true score), LowerBound (≥ this score), UpperBound (≤ this score)
- **Replacement**: depth-preferred + age-based; shallow UpperBound nodes are
  skipped entirely (selective store, cuts ~60% of writes)
- **Allocation**: 64MB with `madvise(MADV_HUGEPAGE)` for reduced TLB pressure
- **Move packing**: from:6 + to:6 + kind:2 + promo:2 = 16 bits

---

## Step 6: Move ordering

Moves are ordered from best to worst by sorting key:

1. **TT move** (if this position is in the TT) — i32::MAX
2. **SEE-winning captures** (SEE > 0) — 10,000 + SEE score
3. **Promotions** — 30,000
4. **Killer moves** (2 per ply) — 9,000 / 8,999
5. **History heuristic** (64×64 table, entries incremented by depth² on cutoff)
6. **SEE-losing captures** (SEE ≤ 0) — 2,000 + SEE score

Static Exchange Evaluation (SEE) replaces raw MVV-LVA for capture ordering.
SEE recursively simulates exchanges on the target square, using the smallest
attacker first, to determine if a capture wins or loses material. Winning
captures get top priority; losing captures are still searched but deprioritized.

Multi-threaded: threads 1..N perturb quiet-move history scores by
`(from_square × thread_id) % 16` for search diversity.

---

## Step 7: Quiescence search

At the horizon (depth 0), instead of returning the static eval immediately, the
engine enters **quiescence search**: search only captures and promotions
recursively until the position is "quiet" (no hanging pieces).
**Stand-pat**: if the static eval is already ≥ beta, return immediately.

Q-search prunes **losing captures** (SEE < 0) — a capture that loses material
without compensation is skipped entirely in the quiet phase.

---

## Step 8: Null move pruning

If a position is so good that even giving the opponent a free move doesn't hurt:
1. Give the opponent an extra (null) move
2. Search with reduced depth (R=3 at depth≥6, R=4 otherwise)
3. If score is still ≥ beta, prune

Skipped when in check or when only pawns + king remain (zugzwang risk).

---

## Step 9: Futility pruning

Near the horizon (depth ≤ 2), skip quiet moves when the static eval is far below
alpha:
- Depth 1: static_eval + 200cp ≤ alpha → prune
- Depth 2: static_eval + 400cp ≤ alpha → prune

Captures, promotions, and checks are never pruned.

---

## Step 10: Pin pre-filter

`Board::pinned_pieces()` ray-scans from the king. At the search level:
- Non-pinned, non-king, non-ep moves skip the expensive make/unmake/is_attacked_by
  legality check entirely — they're trivially legal.

---

## Step 11: Mate handling

When a side has zero legal moves:
- King in check → checkmate. Score = `-CHECKMATE + ply` (prefers shorter mates)
- King not in check → stalemate. Score = 0 (draw).

Mate scores are ply-adjusted when storing/retrieving from the TT.

---

## Step 12: Multi-threaded search (Lazy SMP)

N search threads share the lock-free TT. Each thread runs the same iterative
deepening search independently. No explicit work partitioning — cooperation
through TT sharing. Thread 0 is authoritative; threads 1..N add diversity via
perturbed quiet-move ordering. Scaling: 2 threads beneficial at depth 9+.

---

## Step 13: MultiPV

When `setoption name MultiPV value N` is set (N ≥ 2), the root search collects
N best moves per depth iteration. Each PV index gets its own aspiration window
and excluded-moves list (previously found best moves are skipped).
Output: `info multipv 1 score cp ... pv ...`, `info multipv 2 ...`, etc.

---

## Step 14: Ponder

`go ponder` starts a speculative search during the opponent's turn (infinite
mode, no movetime timer). On `ponderhit` (opponent played the predicted move),
the search continues and reports normally. On `stop` without `ponderhit`, the
search is silently discarded.

---

## Step 15: Opening book

`setoption name OwnBook value true` + `setoption name BookFile value path.bin`
loads a Polyglot `.bin` book file. On `go`, if the book has a move for the
current position (matched by Zobrist hash), it's played immediately — search
skipped entirely. 16-byte entries: hash(8) + move(2) + weight(2) + learn(4).
Binary search, highest-weight move selection.

---

## Step 16: Bitboard slider movegen

All piece types now use bitboard move generation:
- Knights, kings, pawns: precomputed attack tables (compile-time const)
- Sliders (bishop, rook, queen): magic bitboards with runtime-generated tables,
  O(1) attack detection, bitscan extraction for move list
- Verified by exhaustive magic tests (all 4096 blocker subsets for bishops)
  and perft regression tests

---

## Putting it all together

```
function search_worker(board, params, stop, tt, thread_id):
    multi_pv = params.multi_pv.max(1)
    prev_scores[0..multi_pv] = 0
    for depth = 1 .. max_depth:
        for mpv in 0 .. multi_pv:
            setup_excluded_moves(previous PV moves)
            // aspiration window
            alpha = prev_scores[mpv] - delta, beta = prev_scores[mpv] + delta
            score = aspiration_loop:
                score = alpha_beta(depth, alpha, beta, ply=0)
                if fail-low:  widen alpha, re-search
                if fail-high: widen beta, re-search
            collect PV, add to depth_results
            update prev_scores[mpv]
        update best_result from depth_results
        if mate found or stopped: break
    return best_result

function alpha_beta(board, alpha, beta, depth, ply, state, tt, is_pv, thread_id):
    // 0. Stop check + horizon → quiescence
    // 1. Checkmate / stalemate detection
    // 2. TT probe
    // 3. Null move pruning (depth ≥ 3, not in check, has big pieces)
    // 4. Compute static_eval (if depth ≤ 2, for futility pruning)
    // 5. Generate pseudo-legal moves, order by score key
    // 6. For each move (skip excluded at root):
    //    6a. Legality check (pin pre-filter for trivial legality)
    //    6b. Futility pruning (depth ≤ 2, quiet, no check, margin check)
    //    6c. First move: full-window PVS
    //        Rest: null-window PVS + LMR on late quiet non-killers
    //    6d. PV collection on improvement
    //    6e. Beta cutoff → killer move + history update, break
    // 7. Selective TT store (skip shallow UpperBound)
    return best_score

function quiescence(board, alpha, beta, ply, state):
    // 0. Stand-pat cutoff
    // 1. Generate captures + promotions only
    // 2. Prune losing captures (SEE < 0)
    // 3. Recursive q-search on remaining captures
    return alpha
```
