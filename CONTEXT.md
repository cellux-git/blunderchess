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

**Piece-Square Table (PST)**: A static 64-square bonus/malus per piece type. Uses PeSTO's published tables with tapered evaluation (midgame/endgame blending based on material phase). EG king table uses a centralized pattern rewarding king centralization in endgames.

**Mobility**: The number of safe squares a piece can move to. Evaluated via logarithmic tables (diminishing returns per additional square) with separate MG and EG tables. All pieces count enemy-occupied squares as mobile.

**Bad bishop**: A bishop blocked behind its own same-color pawn chain, with low mobility. Penalized per blocking pawn, with extra penalty if the blocking pawn is fixed (cannot advance). Distinct from "trapped bishop" (old term, now subsumed).

**Closed file rook**: A rook on a file where a friendly pawn exists, giving the rook no vertical scope. Penalized.

**7th-rank rook**: A rook on the opponent's second rank (rank 7 for white, rank 2 for black), attacking the opponent's pawn structure. Bonused.

**Rook-queen battery**: Queen and rook aligned on the same file or rank, with bonus scaled by line openness (open > semi-open > closed-movable).

**Queen multi-attack**: Queen attacking multiple enemy pieces simultaneously. Two components: per-piece attack count bonus and fork detection (attacking 2+ undefended pieces).

**Knight passivity**: Penalty for knights on the rim (a/h-file) or trapped (zero safe squares).

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

132 tests across 12 modules (122 unit + 10 integration; all pass):

| Module | Count | Key areas tested |
|--------|-------|-----------------|
| `board.rs` | 14 | Magic tables (exhaustive), make/unmake roundtrip, FEN parsing, castling rights, check/checkmate/stalemate, clone independence |
| `movegen.rs` | 14 | 6 CPW perft positions (d1-3), pinned pieces, en passant discovery, castling through check, double check, promotion underpromotion, stalemate |
| `search.rs` | 13 | Valid move, mate detection, iterative deepening, stop flag, PV collection, TT multi-threading, qsearch capture, draw detection, null move smoke |
| `eval.rs` | 11 | Material + PST, pawn struct (doubled/isolated/passed/backward), bishop pair + bad bishop, rook files (+closed, +7th rank), rook-queen battery, queen multi-attack, outpost knights + rim/trapped, connected passers, candidate passers, passer blocker, rook behind passer, king-passer proximity (MG+EG), mobility (logarithmic, MG+EG), king safety, king opposition, space control, pawn majority, exchange evaluation, tapered MG/EG blend |
| `tt.rs` | 5 | Probe/store roundtrip, misses, depth-preferred replacement, age-based, move pack |
| `types.rs` | 13 | Move packing (all kinds), Square bounds, Color flip, Move::NULL, CastlingRights |
| `uci.rs` | 6 | Parse UCI move roundtrip, position startpos/FEN/moves, go depth, invalid input |
| `zobrist.rs` | 3 | Incremental hash matches full, hash changes after move, side-to-move toggle |
| `tests/benchmarks.rs` | 10 | Tactical: Scholar's Mate, back-rank mate, hanging queen, promotion, smothered mate, mate-in-2, pin, discovered attack, depth convergence. 3 ignored perf benchmarks. |

| # | Task | Status | Impact | Notes |
|---|------|--------|--------|-------|
| 1 | Make/unmake with state stack | ✅ DONE | **High** — eliminates per-node Board clone | |
| 2 | Packed `u16` moves | ✅ DONE | **High** — zero heap allocations in movegen | |
| 3 | Bitboards + magic sliders | ✅ DONE | **High** — O(1) attack detection | Runtime-generated bishop magics |
| 4 | Pseudo-legal movegen | ✅ DONE | **High** — ~25% speedup | Legality check in search |
| 5 | Killer moves + history heuristic | ✅ DONE | **Medium** — ~15% node reduction | 2 killers/depth, 64×64 table |
| 6 | Tapered evaluation | ✅ DONE | **Medium** — better endgame play | mg/eg PST blending by material phase |
| 7 | Lazy SMP (multi-threaded) | ✅ DONE | **High** — TT sharing reduces nodes | Threads 2+ perturb quiet ordering |
| 8 | Lock-free TT + huge pages | ✅ DONE | **High** — 2× vs Mutex TT | Selective store skips ~60% of writes |
| 9 | Aspiration windows | ✅ DONE | **Low** — ~10% speedup | Depth ≥4: narrows root window to prev_score ± 25cp; re-searches with wider bounds on fail-low/high. Neutral at depth ≤10. |
| 10 | Late move reductions (LMR) | ✅ DONE | **Medium** — reduces nodes at moderate+ depths | Reduces quiet moves 4+, skips killers; neutral at depth≤10 |
| 11 | Full positional evaluation | ✅ DONE | **High** — quality jump | 14 evaluation term groups: pawn structure (doubled/isolated/passed/backward/phalanx/chain/candidate), king safety (shield + open files + zone attackers), mobility (logarithmic, MG+EG, 4 piece types), bishop pair + bad bishop (generalized), rook files (+closed, +7th rank, +queen battery), queen multi-attack (+fork), outpost knights (+rim/trapped), connected passers, passer blocker, rook behind passer, king-passer proximity (MG+EG), king opposition, space control, pawn majority, exchange evaluation. |
| 12 | Configurable piece values | ✅ DONE | **Medium** — unlocks tuning | `Eval` struct with 40+ fields: material, PSTs, pawn struct, mobility, king safety; `Eval::default()` returns PeSTO |
| 13 | Pre-filter legal moves (pin detection) | ✅ DONE | **Medium** — eliminates clone + redundant legality checks | `Board::pinned_pieces()` via ray-scan; search uses `generate_pseudo_legal` (no clone); non-pinned non-king non-ep moves skip make/unmake/is_attacked_by. Modest speedup at current depths; scales with branching factor. |
| 14 | Workspace split (lib + bin) | ✅ DONE | **Low** — structural | `src/lib.rs` added; integration tests in `tests/` |
| 15 | Benchmark suite | ✅ DONE | **Low** — validation | 3 ignored perf benches + 10 tactical tests in `tests/benchmarks.rs` |
| 16 | UCI: Ponder, MultiPV | ✅ DONE | **Low** — niche features | `go ponder`/`ponderhit`/`stop` flow; `setoption MultiPV value N`; multi-PV root loop with per-index aspiration windows and move exclusion |
| 17 | Opening book (Polyglot) | ✅ DONE | **Low** — no strength impact | New `src/book.rs`: loads `.bin` files (16-byte entries), binary search by `board.hash`, Polyglot move ↔ `Move` conversion, weighted pick. UCI: `setoption name OwnBook value true`, `setoption name BookFile value path.bin`. |
| 18 | Full bitboard movegen for sliders | ✅ DONE | **Medium** — replace mailbox slider loops | Magic-based bit extraction; ~57 lines removed |
| 19 | Futility pruning | ✅ DONE | **Medium** — reduces nodes near horizon | Skip quiet moves at depth ≤ 2 when static_eval + margin < alpha. Margins: 200cp@d1, 400cp@d2. |
| 20 | Static Exchange Evaluation (SEE) | ✅ DONE | **Medium** — better capture ordering | Recursive SEE (smallest attacker first) in `src/eval.rs`. Replaces MVV-LVA in `order_moves`/`order_moves_q`; losing captures (SEE < 0) pruned in quiescence. 5 unit tests. |
| 21 | Summary + interactive extension of positional eval | ✅ DONE | **Medium** — eval quality | Reviewed `eval.rs`, added 13 new evaluation term groups across 13 `ready-for-agent` issues. All unit + integration tests pass (86). |
| 22 | Thread pool for search workers | ✅ DONE | **Medium** — multicore scaling | Replace per-thread spawn with a persistent thread pool |
| 23 | Fix unsound Nxb4 sacrifice from pawn PST bias | ✅ DONE | **High** — eval quality | Reduced mg_pawn_table row 1 (rank 2/7) from ~100 avg to ~5, increased rows 2–3 for advanced pawns. Nxb4 static eval from +457 cp Black → neutral. |
| 24 | Human review of positional evaluation | `needs-info` | **Medium** — eval quality | Review all eval terms; suggest improvements and new terms |
| 25 | Remove opening-book compensation hacks from knight/bishop/king PST | ✅ DONE | **Medium** — eval quality | Replaced mg_knight (swing -89→+129 → -10→+15), mg_bishop (c1 -82→0), mg_king (e1 -56→-10) with smooth centralization tables. All 142 tests pass. No NPS regression. |
| 26 | Group Eval into six domain sub-structs | ✅ DONE | **High** — testability | ADR-0007. MaterialValues, PieceSquareTables, MobilityTables, PawnEval, PieceEval, KingEval with Default impls. All eval functions updated. |
| 27 | Extract SearchAlgorithmParams from hardcoded magic numbers | ✅ DONE | **Medium** — tunability | ADR-0008. Nested LmrConfig, NullMoveConfig, AspirationConfig, FutilityConfig. 15+ magic numbers → SearchAlgorithmParams::default(). |
| 28 | Encapsulate Engine: private fields, fix ThreadPool lifecycle | ✅ DONE | **Medium** — safety | ADR-0009. All 11 fields private. search_position() added as test seam. ThreadPool no longer replaced at runtime. |
| 29 | Extract MoveOrdering module from search.rs | ✅ DONE | **Medium** — testability | Killer table + history heuristic + order_moves/q extracted to src/move_ordering.rs. ~55 lines removed from search.rs. |
| 30 | Make passed_pawns a free function | ✅ DONE | **Low** — cleanup | Removed dead &self parameter. Extracted from impl Eval to standalone function in pawns.rs. |

## Performance (release build, startpos, 1 thread)

| Depth | Nodes | Time (ms) | NPS |
|-------|-------|-----------|-----|
| 3 | 1,159 | 4 | 290K |
| 4 | 3,520 | 20 | 176K |
| 5 | 25,956 | 95 | 273K |
| 6 | 37,366 | 136 | 275K |
| 7 | 79,409 | 255 | 311K |
| 8 | 189,551 | 703 | 270K |
| 9 | 458,908 | 1,690 | 271K |
| 10 | 2,807,299 | 10,519 | 267K |
| 11 | 11,379,412 | 44,172 | 258K |

Steady ~270K NPS. Branching factor ~4.9x per ply.

## Lazy SMP scaling data

Release build, startpos, depth 10:

| Threads | Nodes | Time (ms) | vs t1 |
|---------|-------|-----------|-------|
| 1 | 2,807,299 | 10,509 | 1.00× |
| 2 | 1,704,735 | 7,939 | 1.32× faster |
| 4 | 1,275,769 | 12,567 | 0.84× slower |

2 threads provides a 1.32× speedup via TT sharing (39% fewer nodes). 4 threads reduces nodes to 45% but cache contention negates the benefit.

Depth sweep 6–11 (pre-eval overhaul):
| Depth | t1 (ms) | t2 (ms) | t4 (ms) | Best |
|-------|---------|---------|---------|------|
| 6 | 131 | 154 | 157 | t1 |
| 9 | 1,679 | 1,613 | 2,452 | ~tie |
| 10 | 10,554 | 9,153 | 12,295 | t2 |
| 11 | 44,835 | 32,371 | — | t2 (1.38×) |
