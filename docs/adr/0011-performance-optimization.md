# Performance optimization

**Status**: accepted

## Context

A chess engine's playing strength depends on search depth, which depends on nodes-per-second (NPS) throughput. The engine must sustain ≥1M NPS at depth 7+ on a single thread to reach meaningful depths within tournament time controls. Multi-threaded Lazy SMP adds scaling pressure — contention on shared data (especially the transposition table) must not destroy per-thread throughput.

Optimization is an ongoing activity, not a one-time project. This ADR records optimization techniques that have been applied, their measured impact, and techniques that were tried and rejected.

## Decision

Optimization efforts target three layers:

1. **Hot-path micro-optimizations** — cache-line alignment, branch prediction, instruction-level parallelism in make/unmake, movegen, and eval.
2. **Algorithmic pruning** — techniques that reduce the node count without weakening play (null move, razoring, futility, delta pruning, SEE pruning, LMR, IIR).
3. **Concurrency** — lock-free data structures, contention reduction, and thread scaling.

### Active optimization techniques

| Technique | Layer | Area | Impact |
|-----------|-------|------|--------|
| 64-byte-aligned TT buckets with 128-byte padding | Concurrency | TT | Eliminates cache-line false sharing between threads |
| `AtomicU64` with Acquire/Release ordering | Concurrency | TT | Lock-free 4-way associative probe/store |
| QS TT stores throttled to Exact/LowerBound only | Concurrency | TT | Reduces atomic contention; UpperBound pollution avoided |
| `madvise` huge pages on TT allocation | Hot-path | TT | Fewer TLB misses on large TT lookups |
| Pinned bitboards cached incrementally in make/unmake | Hot-path | Movegen | Avoids recomputing pin masks per node |
| Phase (midgame/endgame) cached incrementally | Hot-path | Eval | Avoids recomputing material phase in eval |
| Trivial-legality pre-filter before make/unmake | Hot-path | Search | Most moves skip expensive is_attacked_by check |
| Stop flag amortization (clock check every 1024 nodes) | Hot-path | Search | Reduces `Instant::elapsed()` syscalls ~1000× |
| Cache `in_check` at alpha_beta entry | Hot-path | Search | Avoids 2 redundant `is_attacked_by` calls per node |
| `color_at(to)` → `color.flip()` in capture branch | Hot-path | Board | Eliminates mailbox lookup in capture |
| `CASTLE_LOSE_MASK[64]` lookup table | Hot-path | Board | Replaces 8 `if/else if` with 2 table lookups |
| Packed `CastlingRights(u8)` + table-lookup zobrist | Hot-path | Zobrist | Single u8 index → precomputed hash, no branches |
| `rank_mask_forward` compile-time LUT | Hot-path | Eval | Replaces per-pawn loop with array lookup |
| `adjacent_files_mask` compile-time LUT | Hot-path | Eval | Replaces per-pawn branch with array lookup |
| Split `is_attacked_by` slider guard | Hot-path | Board | Skips bishop lookup when no bishops/queens exist |
| Duplicate AND elimination in eval accumulate macro | Hot-path | Eval | Single `& us_bb` used for both count and iteration |
| `sort_by_score_desc_with_flags` for QS | Hot-path | MoveOrdering | Parallel-array sort preserves legality flags (unused after QS flag revert) |
| Delta pruning in QS | Algorithmic | Q-search | Skips captures when stand-pat + 900 ≤ alpha |
| Razor pruning at frontier | Algorithmic | Search | Prunes quiet moves at depth 1-3 below alpha |
| Futility pruning at frontier | Algorithmic | Search | Skips quiet moves at depth 1-2 with poor static eval |
| Null move pruning with adaptive R | Algorithmic | Search | Doubles effective depth reach |
| History-based LMR with gravity aging | Algorithmic | Search | Reduces late-move search depth, history guides selection |
| IIR on TT misses | Algorithmic | Search | Reduced-depth search ensures good move ordering |
| Killer moves (2 per depth) | Algorithmic | Search | Improves move ordering for non-capture cutoff |
| SEE-based capture ordering | Algorithmic | Search | Better capture ordering than MVV-LVA |

### Accepted optimizations (2026-06-18 session)

| # | Optimization | Files | Single-thread Δ | Multi-thread Δ |
|---|-------------|-------|-----------------|----------------|
| 1 | Stop flag amortization | `worker.rs` | +5-10% | +5% |
| 2 | Fixed-size `[u64; 100]` ring buffer for history | `board.rs`, `draw.rs` | +5% | **+90% at 16T** |
| 3 | Cache `in_check` at alpha_beta entry | `alpha_beta.rs` | +3% | +3% |
| 4 | `color_at(to)` → `color.flip()` in capture | `board.rs` | marginal | marginal |
| 5 | `CASTLE_LOSE_MASK[64]` lookup table | `board.rs`, `types.rs` | marginal | marginal |
| 6 | Packed `CastlingRights(u8)` + table-lookup zobrist | `types.rs`, `zobrist.rs`, `movegen.rs` | marginal | marginal |
| 7 | `rank_mask_forward` LUT | `attack.rs` | **+15%** | **+15%** |
| 8 | `adjacent_files_mask` LUT | `attack.rs` | **+15%** | **+15%** |
| 9 | Removed pointless `if` branch in eval same_rank | `pawns.rs` | negligible | negligible |
| 10 | Split `is_attacked_by` slider guard | `board.rs` | +3% | +3% |
| 11 | Phase clamp redundant cast removed | `board.rs` | negligible | negligible |
| 12 | Dead `king_bb & us_bb == 0` guard removed | `movegen.rs` | negligible | negligible |
| 13 | Battery between-loop → bit arithmetic | `eval/pieces.rs` | negligible | negligible |
| 14 | Duplicate AND in eval accumulate macro | `eval/mod.rs` | negligible | negligible |
| 15 | TT prefetch for child after make_move | `alpha_beta.rs`, `quiescence.rs` | marginal | marginal |

### Current performance baseline

See `CONTEXT.md` for full benchmark tables. Summary:

- **Single-thread NPS**: ≥1.4M at depth 7+, peaking at 1.44M at depth 8 (warm TT, startpos)
- **Perft speed**: 5.76M NPS at depth 3 (kiwipete)
- **Lazy SMP scaling**: 10.2× speedup at 16 threads (depth 8, cold TT), 63% efficiency
- **Deep scaling**: 11.4M NPS at depth 10 with 16 threads

## Approaches that did not work out

### MVV-LVA move ordering (2026-06-18)

**Expected**: Replace SEE (recursive exchange evaluation) with cheap MVV-LVA (10 × victim_material − attacker_material) for capture move ordering. Expected to save significant time per node since SEE is called for every capture, most of which are never searched.

**Result**: Broke `tactical_avoid_queen_trap` and `tactical_avoid_pawn_fork` integration tests. MVV-LVA doesn't account for X-ray attacks, pinned pieces, or multi-exchange sequences.

**Why rejected**: SEE captures subtle tactical nuances (e.g., a queen capture that looks winning but loses to a discovered attack). MVV-LVA is too coarse for accurate capture ordering. The performance gain (~5-10% NPS) wasn't worth the tactical regression.

### QS legality flag array (2026-06-18)

**Expected**: Track trivially-legal moves with a `[bool; MAX_MOVES]` array during QS filtering, then skip the redundant `is_attacked_by` check in the search loop. Expected to save one magic bitboard lookup per QS move.

**Result**: NPS dropped from 1.44M to 918K at depth 8 (36% regression). The 218-byte array allocation per QS node and the extra branches in the insertion sort (now sorting 3 parallel arrays: moves, scores, flags) outweighed the savings.

**Why rejected**: Stack allocation overhead + sort complexity exceeds the benefit for the typical QS move count (~5-30 moves). The `is_attacked_by` call is only expensive when slider pieces are present; many QS positions have few or no sliders, so the savings were smaller than expected. The extra 218-byte stack frame on every QS node dominated.

### Batch pawn attack mask (2026-06-18)

**Expected**: Replace per-pawn `pawn_attacks()` table-lookup loop in `enemy_pawn_attack_mask` with 4 batch bitwise shifts. Expected O(1) instead of O(pawns). Called 2× per `evaluate()`.

**Result**: NPS dropped from 1.44M to 954K at depth 8 (34% regression). The 4 × 64-bit shifts + 2 NOTs per call were slower than the per-pawn loop (2-8 iterations of trailing_zeros + table lookup + OR).

**Why rejected**: The compiler already optimized the per-pawn loop well (likely unrolling for common pawn counts). The shift approach added constant overhead that dominated for typical pawn counts. No measurable gain — the per-pawn approach is already near-optimal.

## Extension path

New optimization techniques should be:
1. Benchmarked with `cargo test --release --test benchmarks -- --ignored --nocapture` before and after
2. Recorded in the appropriate table above if accepted
3. Recorded in "Approaches that did not work out" if rejected, with measurements

### Remaining candidates (measured, not yet attempted)

| # | Opportunity | File | Likely gain | Risk |
|---|------------|------|-------------|------|
| 1 | Staged move generation (captures first, quiets only if no cutoff) | `alpha_beta.rs` | MEDIUM | MEDIUM — complex search refactor |
| 2 | Pin recomputation via directional shifts or ray tables | `board.rs` | LOW | MEDIUM — pin computation bugs cause illegal moves |
| 3 | `order_moves_q` gives_check via bitboard without make/unmake | `move_ordering.rs` | LOW-MED | MEDIUM — discovered-check detection is non-trivial |
| 4 | Lazy piece_list update (only rebuild when needed for FEN) | `board.rs` | LOW | LOW — piece_list is O(32) scan per move |

## Consequences

- TT bucket alignment and huge pages require `std::alloc` with custom layout; this is platform-specific (Linux `madvise`) and would need porting for other OSes.
- Incremental cached bitboards (pinned, phase) add state to `Board` and complexity to `make_move`/`unmake_move`; any bug here silently corrupts search results.
- QS TT store throttling (Exact/LowerBound only) is a tradeoff — UpperBound entries in QS could improve accuracy but increase contention.
- The fixed-size history array (`[u64; 100]`) caps the game length at 100 half-moves for repetition detection; this matches the 50-move rule and is sufficient for all practical use. If exceeded, history silently wraps, which is safe (older positions can't cause a threefold repetition with the current position).
