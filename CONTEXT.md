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

**Piece-Square Table (PST)**: A static 64-square bonus/malus per piece type. Uses PeSTO's published tables with tapered evaluation (midgame/endgame blending based on material phase).

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

- **`Eval` struct**: Holds piece values and PST arrays as fields. Construct with defaults (PeSTO) or custom values for tuning.
- **`SearchParams` struct**: All tunable constants (null-move R, etc.) as fields. Pass by reference.
- **`Engine` facade**: Wires Board + Eval + Search + TT + UCI. Public entry point for integration tests.

## Test coverage

89 tests across 9 modules (79 unit + 10 integration; all pass):

| Module | Count | Key areas tested |
|--------|-------|-----------------|
| `board.rs` | 14 | Magic tables (exhaustive), make/unmake roundtrip, FEN parsing, castling rights, check/checkmate/stalemate, clone independence |
| `movegen.rs` | 14 | 6 CPW perft positions (d1-3), pinned pieces, en passant discovery, castling through check, double check, promotion underpromotion, stalemate |
| `search.rs` | 13 | Valid move, mate detection, iterative deepening, stop flag, PV collection, TT multi-threading, qsearch capture, draw detection, null move smoke |
| `eval.rs` | 11 | Material + PST, pawn struct (doubled/isolated/passed/backward), bishop pair + trapped bishop, rook files, outpost knights, connected passers, rook behind passer, king-passer proximity, mobility, king safety, tapered MG/EG blend |
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
| 11 | Full positional evaluation | ✅ DONE | **High** — quality jump | Pawn structure, king safety (shield + open files + zone attackers), mobility (N/B/R/Q), bishop pair + trapped bishops, rook files, outpost knights, connected passers, rook behind passer, king-passer proximity. Fixed double phase-weighting bug. |
| 12 | Configurable piece values | ✅ DONE | **Medium** — unlocks tuning | `Eval` struct with 40+ fields: material, PSTs, pawn struct, mobility, king safety; `Eval::default()` returns PeSTO |
| 13 | Pre-filter legal moves (pin detection) | ✅ DONE | **Medium** — eliminates clone + redundant legality checks | `Board::pinned_pieces()` via ray-scan; search uses `generate_pseudo_legal` (no clone); non-pinned non-king non-ep moves skip make/unmake/is_attacked_by. Modest speedup at current depths; scales with branching factor. |
| 14 | Workspace split (lib + bin) | ✅ DONE | **Low** — structural | `src/lib.rs` added; integration tests in `tests/` |
| 15 | Benchmark suite | ✅ DONE | **Low** — validation | 3 ignored perf benches + 10 tactical tests in `tests/benchmarks.rs` |
| 16 | UCI: Ponder, MultiPV | ✅ DONE | **Low** — niche features | `go ponder`/`ponderhit`/`stop` flow; `setoption MultiPV value N`; multi-PV root loop with per-index aspiration windows and move exclusion |
| 17 | Opening book (Polyglot) | ✅ DONE | **Low** — no strength impact | New `src/book.rs`: loads `.bin` files (16-byte entries), binary search by `board.hash`, Polyglot move ↔ `Move` conversion, weighted pick. UCI: `setoption name OwnBook value true`, `setoption name BookFile value path.bin`. |
| 18 | Full bitboard movegen for sliders | ✅ DONE | **Medium** — replace mailbox slider loops | Magic-based bit extraction; ~57 lines removed |
| 19 | Futility pruning | ✅ DONE | **Medium** — reduces nodes near horizon | Skip quiet moves at depth ≤ 2 when static_eval + margin < alpha. Margins: 200cp@d1, 400cp@d2. |
| 20 | Static Exchange Evaluation (SEE) | ✅ DONE | **Medium** — better capture ordering | Recursive SEE (smallest attacker first) in `src/eval.rs`. Replaces MVV-LVA in `order_moves`/`order_moves_q`; losing captures (SEE < 0) pruned in quiescence. 5 unit tests. |
| 21 | Summary + interactive extension of positional eval | ☐ TODO | **Medium** — eval quality | Review current `eval.rs` (tapered, 762 lines: material, PST, pawn struct, king safety, mobility, bishop pair, rook files, outpost knights, passers, trapped bishops) and surface missing terms with user for guided improvement. |

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
