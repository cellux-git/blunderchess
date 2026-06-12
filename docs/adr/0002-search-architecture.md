# Alpha-beta search with iterative deepening, transposition table, and quiescence search

**Status**: accepted

## Context

A chess engine's search algorithm explores the game tree to find the best move. The design space ranges from simple 1-ply material counting to full N-ply alpha-beta with dozens of heuristics.

## Decision

The v1 search is **alpha-beta with Principal Variation Search (PVS), iterative deepening, a transposition table, quiescence search (captures only, stand-pat), and null move pruning**. Mate scores are ply-adjusted. The principal variation is collected via a triangular PV array.

## Why

- Alpha-beta is the universal foundation. Every chess engine uses it. Adding PVS (full-window on first move, null-window on rest) is a 10-line refinement that yields 20-40% fewer nodes.
- Iterative deepening solves time management naturally (search until the time budget expires, use the last completed iteration's result) and improves move ordering for deeper searches (shallower results seed the TT).
- A transposition table is necessary for iterative deepening to be effective — without it, each iteration restarts from scratch. The TT also caches results across different move orders reaching the same position.
- Quiescence search prevents the "horizon effect" (mis-evaluating positions where a capture is pending just beyond the search horizon). Captures-only with stand-pat is the standard minimum.
- Null move pruning adds massive depth reach (~doubles effective depth) for ~15 lines and has no interaction with other heuristics in this setup.
- PVS and null move pruning are bolt-on enhancements to alpha-beta, not separate algorithms. Including them in v1 avoids needing to retrofit the search loop later.

## Extension path

All planned search extensions have been implemented: killer moves + history heuristic, Late Move Reductions (LMR), aspiration windows, pin pre-filter, futility pruning, Static Exchange Evaluation (SEE) for capture ordering, bitboard slider movegen, Lazy SMP multi-threading, MultiPV, ponder, and Polyglot opening book. See `CONTEXT.md` for the full task list.

## Considered options

| Option | Rejected because |
|--------|------------------|
| Bare alpha-beta without ID/TT | Time control becomes "guess a depth." Without TT, iterative deepening is pointless. The engine is unusable in real games. |
| PVS and null move as v2 additions | The code cost is tiny (~25 lines total). Retrofitting them into the search loop is riskier than building with them from the start — they touch the core recursive logic. |
| Quiescence with checks included | Checks in q-search explode the tree (check evasions branch widely). Standard engines either handle checks with SEE or defer them. |

## Consequences

- The search implementation is ~300-400 lines of recursive code. Correctness depends heavily on perft-passing move generation and Zobrist correctness (every bug in either produces silent search errors).
- TT entries must correctly encode node type (exact, lower bound, upper bound) and adjust mate scores by depth. These are standard but easy to get wrong.
- Search speed is sufficient for depth 8-12 in typical midgame positions with 64MB TT, which is adequate for legal play and beating beginners.
