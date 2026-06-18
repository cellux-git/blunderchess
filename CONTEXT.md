# BlunderChess

A hobby chess engine. Uses mailbox board with bitboard attack detection, piece-square table evaluation with tapered midgame/endgame blending, alpha-beta search with Lazy SMP.

## Language

**Board**: The 8×8 chess board. Represented as a `[Option<Piece>; 64]` mailbox array plus `pieces_bb[6]`, `colors_bb[2]`, and `occupancy` bitboard fields for O(1) attack detection.
_Avoid_: Position, game-state (Board specifically means the square + piece layout)

**Mailbox**: Array-based board representation. Each square index (0..63) holds either a piece or nothing. Augmented with bitboard fields.
_Avoid_: 0x88 (related mailbox variant, not used)

**Move**: A packed `u16`: `from:6 + to:6 + kind:2 + promo:2`. En passant detected from context (pawn capture to empty ep square). MoveKind has 4 variants: Normal, Capture, Castle, Promotion.

**Legal move**: A move that does not leave the moving side's king in check. Movegen produces pseudo-legal moves; the search loop pre-filters trivially-legal moves (non-pinned, non-king, non-EP, non-castle) and only runs make/unmake/is_attacked_by for the remaining edge cases. Perft uses the full `generate_legal_moves` filter.

**Make move / unmake move**: `Board::make_move(&mut self) -> UndoInfo`, `unmake_move(&undo)`. Stack-allocated `UndoInfo` stores all state needed to reverse the move. No per-node Board clone.

**Search**: Alpha-beta with PVS, iterative deepening, quiescence search (captures only with stand-pat), null move pruning, killer moves (2 per depth), and history heuristic (64×64 table).

**Quiescence search** ("q-search"): A restricted search that only explores captures at the horizon, preventing the engine from thinking an arbitrary capture sequence ends the line. Uses stand-pat (return static eval if it already beats beta).

**Iterative deepening**: Searching to depth 1, then 2, 3, ... until time runs out. Each completed iteration provides a result immediately; the search is interruptible via a stop flag. Enables time management.

**Transposition table (TT)**: Lock-free hash table mapping Zobrist hash → packed entry. 3× `AtomicU64` per bucket with Acquire/Release ordering. Depth-preferred + age-based replacement. Selective store (skip depth-1 fail-lows). Huge pages via `madvise` on the 64MB allocation.

**Principal Variation (PV)**: The sequence of best moves found by the search. Collected via a triangular PV array during search.

**Zobrist hash**: A 64-bit hash of a board position, computed by XORing pre-generated random keys for each (piece, color, square) combination, plus side-to-move, castling rights, and en passant file. Updated incrementally during make_move. Generated at compile time via a const-fn LCG.

**Piece-Square Table (PST)**: A static 64-square bonus/malus per piece type. Encodes the **development principle**: pieces on central, advanced squares score higher; pieces on starting squares score lower. The pawn PST gradient must be smooth enough that passive pawn pushes don't overwhelm piece development. Uses PeSTO's published tables with tapered evaluation (midgame/endgame blending based on material phase). EG king table uses a centralized pattern rewarding king centralization in endgames. Row 2 of mg_pawn_table was moderated (from max 65 to max 35) to prevent aggressive pawn advancement from dominating development moves.

**Mobility**: The number of safe squares a piece can move to. Evaluated via logarithmic tables (diminishing returns per additional square) with separate MG and EG tables. Safe = not occupied by friendly piece AND not attacked by enemy pawn. This is a principle: piece activity matters, not which specific squares are attacked.

**Bad bishop**: A bishop blocked behind its own same-color pawn chain, with low mobility. Penalized per blocking pawn, with extra penalty if the blocking pawn is fixed (cannot advance). Distinct from "trapped bishop" (old term, now subsumed).

**Closed file rook**: A rook on a file where a friendly pawn exists, giving the rook no vertical scope. Penalized.

**7th-rank rook**: A rook on the opponent's second rank (rank 7 for white, rank 2 for black), attacking the opponent's pawn structure. Bonused.

**Rook-queen battery**: Queen and rook aligned on the same file or rank, with bonus scaled by line openness (open > semi-open > closed-movable).

**Queen multi-attack**: Queen attacking multiple enemy pieces simultaneously. Two components: per-piece attack count bonus and fork detection (attacking 2+ undefended pieces).

**Knight passivity**: Penalty for knights on the rim (a/h-file) or trapped (zero safe squares).

**Knight outpost**: A knight in the enemy half on a square unreachable by enemy pawns, defended by a friendly pawn. Bonused. The defense requirement is strict — the defending pawn must actually attack the knight's square, not just exist on an adjacent file.

**Evaluation philosophy**: Evaluation uses a handful of generic, principled terms (mobility, king safety, pawn structure, outpost, etc.) rather than many specific rules. When a position is misevaluated, the response is to tighten the relevant principle, not add a new term. See ADR-0010.

**Pawn chain**: Two connected defended pawns. A phalanx (side by side on same rank) and a chain (diagonally defended) both receive bonuses.

**Candidate passer**: A pawn that can become passed by capturing one enemy pawn on an adjacent file. Receives a partial passer bonus.

**Passer blocker**: A friendly piece occupying the square directly in front of an enemy passed pawn, blocking its advance. Bonused.

**King opposition**: When the two kings face each other on the same file or rank with exactly one square between them in an endgame with no remaining enemy Q/R/B/N. Bonused when the opponent cannot waste a tempo (has no safe pawn move).

**Space control**: Bonus for own pawns advanced into the opponent's half (ranks 4-6 for white, ranks 3-5 for black).

**Pawn majority**: Bonus for having more pawns than the opponent on a wing (queenside files a-d, kingside files e-h).

**Exchange evaluation**: Positional adjustment when one side is up the exchange (rook vs minor piece). Factors: open files (rook benefit), opponent bishop pair (rook penalty), and minor piece activity (bonus if opponent's minor is passive).

**Backward pawn**: A pawn whose advance square is attacked by an enemy pawn or blocked, with no friendly pawns on adjacent files behind to support its advance. Penalized.

**Tapered evaluation**: Blends midgame (MG) and endgame (EG) scores based on material phase (0-24). MG weighted by phase, EG weighted by (24-phase). Applied at the top level after all per-side evaluation.

**UCI**: Universal Chess Interface — a text protocol over stdin/stdout for communicating with chess GUIs and tools. BlunderChess implements the full UCI specification.

**Perft**: Performance test — counts all legal leaf nodes at a given depth from a known position. Compares against published CPW perft numbers to validate move generation correctness. All 6 standard CPW positions pass depths 1-3.

**Checkmate / Stalemate**: When a side has no legal moves. Checkmate = king in check, score is -CHECKMATE + ply. Stalemate = king not in check, score is 0 (draw).

**Draw detection**: Threefold repetition (Zobrist hash history comparison), 50-move rule (halfmove clock ≥ 100), and insufficient material (K vs K, K+B vs K, K+N vs K).

## Architecture

### Module dependency graph

```
types ──────────────────────────────────────────────┐
  │                                                  │
zobrist                                               │
  │                                                  │
board (depends: types, zobrist)                       │
  │                                                  │
movegen (depends: board, types)                       │
  │                                                  │
eval (depends: board, types)  ──┐                    │
  │                             │                    │
tt (depends: zobrist, types) ──┤                    │
  │                             │                    │
search (depends: board, movegen, eval, tt) ──┐       │
  │                                           │       │
uci (depends: board, search)                  │       │
  │                                           │       │
main (depends: uci)                           │       │
                                               │       │
All depend transitively on types ◄────────────┘───────┘
```

### Key interfaces

| Boundary | Signature | Notes |
|----------|-----------|-------|
| Movegen | `fn generate_moves(board: &Board, moves: &mut [Move; 218]) -> usize` | Pseudo-legal, stack buffer |
| Eval | `fn evaluate(board: &Board) -> i32` | Free function, PST + material + tapered blending |
| Search | `fn search(board: &Board, params: &SearchParams, stop: &AtomicBool) -> SearchResult` | Lazy SMP worker |
| TT | `fn probe(tt: &TT, hash: u64) -> Option<TTEntry>` / `fn store(tt: &TT, hash: u64, entry: TTEntry)` | Lock-free, `&self` only |
| Make/Unmake | `fn make_move(&mut self, mv: Move) -> UndoInfo` / `fn unmake_move(&mut self, undo: &UndoInfo)` | In-place, no clone |

### Threading model

| Thread | Role |
|--------|------|
| **Main (I/O)** | Reads stdin, parses UCI commands, dispatches handlers. Owns the stop flag (`Arc<AtomicBool>`) and TT (`Arc<TT>`). |
| **Search workers (1..N)** | Spawned on `go`. All run identical iterative deepening; share the lock-free TT for implicit work distribution (Lazy SMP). Thread 0 is authoritative for reported results. Threads 1..N perturb quiet move scores for diversity. |

The I/O thread flips the stop flag on `stop` and joins all search threads before printing `bestmove`. UCI option `go threads N` controls worker count.

### Iteration hooks

- **`Eval` struct**: Holds all tunable evaluation parameters, organized into six sub-structs:
  - **MaterialValues** — piece material values (pawn, knight, bishop, rook, queen, king)
  - **PieceSquareTables** — PST arrays (MG/EG × 6 pieces)
  - **MobilityTables** — logarithmic mobility tables (MG/EG × N,B,R,Q)
  - **PawnEval** — pawn structure (doubled, isolated, passed, backward, phalanx, chain, candidate, blocker), space control, pawn majority
  - **PieceEval** — bishop pair, bad bishop, rook files/open/closed/7th/battery, knight outpost/rim/trapped, queen multi-attack/fork, exchange evaluation
  - **KingEval** — king shield, king open file, king opposition, king-passer proximity, connected passer, rook-behind-passer
  Construct with defaults or custom values for tuning. Each sub-struct is independently testable.
- **`SearchParams` struct**: UCI-level options (depth, movetime, infinite, threads, multi_pv, ponder). Pass by reference.
- **`SearchAlgorithmParams` struct**: Algorithmic tuning knobs, nested into **LmrConfig** (min depth, move threshold, reduction table), **NullMoveConfig** (min depth, R values), **AspirationConfig** (initial delta, depth threshold), and **FutilityConfig** (max depth, margins). Passed alongside SearchParams into search.
- **`MoveOrdering` struct**: Owns killer-move table (2 slots/depth) and history heuristic (64×64 table). Provides `order_moves()` and `order_moves_q()`. Testable independently of alpha-beta.
- **`Engine` facade**: Wires Board + Eval + Search + TT + UCI behind a single public entry point (`process_command`). Internal state is private; integration tests use `search_position(board, depth)`.

## Test coverage

136 tests across 12 modules (126 unit + 10 integration; all pass):

| Module | Count | Key areas tested |
|--------|-------|-----------------|
| `board.rs` | 14 | Magic tables (exhaustive), make/unmake roundtrip, FEN parsing, castling rights, check/checkmate/stalemate, clone independence |
| `movegen.rs` | 14 | 6 CPW perft positions (d1-3), pinned pieces, en passant discovery, castling through check, double check, promotion underpromotion, stalemate |
| `search.rs` | 15 | Valid move, mate detection, iterative deepening, stop flag, PV collection, TT multi-threading, qsearch capture, draw detection, null move smoke, Bb4+ knight trap avoidance, passive f7f6 avoidance |
| `eval.rs` | 13 | Material + PST, pawn struct (doubled/isolated/passed/backward), bishop pair + bad bishop, rook files (+closed, +7th rank), rook-queen battery, queen multi-attack, outpost knights (+rim/trapped, +requires pawn defense), connected passers, candidate passers, passer blocker, rook behind passer, king-passer proximity (MG+EG), mobility (logarithmic, MG+EG), king safety, king opposition, space control, pawn majority, exchange evaluation, tapered MG/EG blend, development-vs-passive-pawn-push |
| `tt.rs` | 5 | Probe/store roundtrip, misses, depth-preferred replacement, age-based, move pack |
| `types.rs` | 13 | Move packing (all kinds), Square bounds, Color flip, Move::NULL, CastlingRights |
| `uci.rs` | 6 | Parse UCI move roundtrip, position startpos/FEN/moves, go depth, invalid input |
| `zobrist.rs` | 3 | Incremental hash matches full, hash changes after move, side-to-move toggle |
| `tests/benchmarks.rs` | 10 | Tactical: Scholar's Mate, back-rank mate, hanging queen, promotion, smothered mate, mate-in-2, pin, discovered attack, depth convergence. 3 ignored perf benchmarks. |

## Performance (release build, startpos, 1 thread, shared TT)

| Depth | Nodes | Time (ms) | NPS |
|-------|-------|-----------|-----|
| 3 | 1,606 | 7 | 229K |
| 4 | 4,311 | 28 | 154K |
| 5 | 8,270 | 52 | 159K |
| 6 | 43,544 | 339 | 128K |
| 7 | 49,563 | 374 | 133K |
| 8 | 348,997 | 2,929 | 119K |
| 9 | 190,363 | 1,606 | 119K |
| 10 | 2,989,937 | 26,605 | 112K |

Steady ~120K NPS. Node counts wobble at depth 7-9 due to shared TT (shallower iterations fill the table, reducing deeper search work).

## Lazy SMP scaling data

Release build, startpos, depth 8, fresh TT per run:

| Threads | Nodes | Time (ms) | vs t1 |
|---------|-------|-----------|-------|
| 1 | 485,444 | 3,923 | 1.00× |
| 2 | 331,859 | 2,857 | 1.37× faster |
| 4 | 331,422 | 2,884 | 1.36× faster |

2-4 threads provide a ~1.37× speedup at depth 8. TT sharing reduces total nodes (32% fewer with 2+ threads).

## Perft speed (kiwipete, release)

| Depth | Nodes | Time (ms) | NPS |
|-------|-------|-----------|-----|
| 1 | 48 | <1 | — |
| 2 | 2,039 | <1 | — |
| 3 | 97,862 | 20 | 4.9M |
