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

### Accepted optimizations (2026-06-19 session)

| # | Optimization | Files | Single-thread Δ | Multi-thread Δ |
|---|-------------|-------|-----------------|----------------|
| 16 | Direct PST indexing (remove Square roundtrip) | `eval/mod.rs` | noise | noise |
| 17 | Pre-extract piece bitboards, pass to sub-evaluators | `eval/mod.rs`, `eval/pieces.rs`, `eval/mobility.rs` | **+50-75% at d3-6**, ~0% at d7-8 | neutral |
| 18 | Const `SQUARES: [Square; 64]` lookup table | `types.rs`, `movegen.rs` | ~1-2% at d7-8, +11% perft | neutral |
| 19 | `excluded_moves` Vec → fixed `[Move; 8]` array | `worker.rs`, `alpha_beta.rs` | noise | noise |

### Current performance baseline

See `CONTEXT.md` for full benchmark tables. Summary (2026-06-19, after optimizations 16-19 + pin-recomputation refinement):

- **Single-thread NPS**: ≥1.3M at depth 7+, peaking at 2.13M at shallow depths (startpos, warm TT).
- **Perft speed**: 10.9M NPS at depth 3 (kiwipete).
- **Lazy SMP scaling**: 11.3M total NPS at 16 threads (depth 8, cold TT).
- **Deep scaling**: 7.94M NPS at depth 10 with 16 threads.

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

### PST roundtrip elimination (2026-06-19)

**Expected**: Eliminate `Square::new(idx).unwrap()` → `pst_value(sq)` → `sq.index()` roundtrip in `accumulate!` macro by passing raw index to a new `pst_value_raw()` method.

**Result**: Measured within noise (depth 7: 1.85M → 1.85M NPS). The compiler already eliminated the bounds check and wrapper entirely via inlining.

**Verdict**: Accepted on cleanliness grounds (removes dead code path), but no measurable NPS gain. The old `pst_value(sq: Square)` method was removed as dead code.

### Lazy SEE computation + partial sort (2026-06-19)

**Expected**: Defer SEE computation to avoid calculating it for captures that are never tried (cut off by beta), and use pick-best instead of full insertion sort.

**Result**: Not implemented. Analysis showed SEE is only ~10-15 captures/node and insertion sort O(n²/2) with n≈40 moves is already minimal. MVV-LVA was previously rejected (breaks tactical tests), and SEE must precede ordering to maintain search quality. Skipped as marginal without architectural change.

### TT Relaxed atomic ordering (2026-06-19)

**Expected**: Use `Ordering::Relaxed` instead of `Acquire` for hash slot probing, reducing memory barriers.

**Result**: Not implemented. On x86-64, `Acquire` and `Relaxed` loads generate identical machine code (the hardware already provides acquire semantics). Would only benefit ARM targets. Skipped.

### Cache rook/bishop attacks for queen mobility (2026-06-19)

**Expected**: Reuse rook/bishop attack bitboards computed during rook/bishop mobility evaluation for the subsequent queen mobility evaluation.

**Result**: Not applicable. Queens are on different squares than rooks/bishops, so their magic bitboard lookups are independent. The existing `queen_attacks` already inlines both rook and bishop lookups. Skipped.

### TT bucket reduction to 64 bytes (2026-06-19)

**Expected**: Reduce PADDED_U64S from 16 to 8 (128→64 byte buckets, 2 slots instead of 4) to improve cache locality.

**Result**: Not implemented. Tradeoff analysis: fewer slots increase evictions; the 4-slot bucket at 128 bytes fits 2 cache lines and is already optimized for parallel probing. Skipped as high-risk without clear benefit.

### Merge mobility into PST accumulation loop (2026-06-19)

**Expected**: Compute mobility bonuses in the same loop as PST accumulation, avoiding a second pass over all piece bitboards.

**Result**: Not implemented. Mobility requires `occ` for slider attack computation, which differs from PST (just board positions). The two passes have different data dependencies and merging would complicate both. Skipped as high-effort, unclear benefit.

### Conditional pinned-piece recomputation (2026-06-19)

**Original approach**: Only recompute `pinned[color]` in `make_move` when the moved piece is the king or was previously pinned (`if piece == Piece::King || (from.bit() & self.pinned[color]) != 0`). Avoids O(8-ray) recomputation on every move.

**Initial result**: Reverted. The conditional update missed the case where a non-pinned piece moves **into** a pin axis (e.g. a knight stepping onto the file between its king and an enemy rook). The stale `pinned` bitboard then caused `alpha_beta`'s `is_trivially_legal` pre-filter to skip the full `is_attacked_by` legality check, allowing illegal moves into the PV. This produced "Illegal PV move" warnings from GUIs. Always-recomputing was the safe fix but caused a 36% perft regression (10.9M→7.0M).

**Refined approach (2026-06-19)**: Recompute `pinned[color]` only when the moved piece could affect pin geometry. A piece can only create or destroy a pin if it was on, or arrived on, a square sharing the king's rank, file, or diagonal:

```rust
if piece == Piece::King || (from.bit() & self.pinned[color]) != 0 {
    self.pinned[color.index()] = self.compute_pinned_impl(color);
} else {
    let ks = self.king_square[color.index()];
    let kf = ks.file() as i32;
    let kr = ks.rank() as i32;
    let on_axis = |sq: Square| -> bool {
        let f = sq.file() as i32;
        let r = sq.rank() as i32;
        f == kf || r == kr || (f - kf).abs() == (r - kr).abs()
    };
    if on_axis(from) || on_axis(to) {
        self.pinned[color.index()] = self.compute_pinned_impl(color);
    }
}
```

This covers all cases:
- King moved (always recompute)
- Previously-pinned piece moved (pin may be dissolved or changed)
- Piece moved FROM a pin axis (may have been blocking a pin for pieces behind it)
- Piece moved TO a pin axis (may have entered a pin — the original bug)

**Result**: Correctness restored. Perft speed recovered to 10.9M NPS. Search NPS unchanged (within noise). Added regression tests for both "moving into pin axis" and "first-blocker moves away" scenarios. The candidate "directional shift" approach (#2 under remaining candidates) could further accelerate `compute_pinned_impl` itself, but the current triggering logic is both correct and cheap.

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

## Consequences

- TT bucket alignment and huge pages require `std::alloc` with custom layout; this is platform-specific (Linux `madvise`) and would need porting for other OSes.
- Incremental cached bitboards (pinned, phase) add state to `Board` and complexity to `make_move`/`unmake_move`; any bug here silently corrupts search results. (See "Conditional pinned-piece recomputation" above — a conditional pin update caused illegal PV moves.)
- QS TT store throttling (Exact/LowerBound only) is a tradeoff — UpperBound entries in QS could improve accuracy but increase contention.
- The fixed-size history array (`[u64; 100]`) caps the game length at 100 half-moves for repetition detection; this matches the 50-move rule and is sufficient for all practical use. If exceeded, history silently wraps, which is safe (older positions can't cause a threefold repetition with the current position).
