# 01 — Add check extensions to alpha-beta

**Category:** `enhancement`
**Status:** `needs-triage`

## Problem

When the engine gives check near the search horizon, the opponent's response is evaluated at `depth - 1`, which means the search stops after the evasion move. Any tactical consequences of the check (captures, follow-up threats beyond the evasion) fall into quiescence search, which only sees captures and checks — missing quiet threats. This is the "horizon effect" applied specifically to check positions.

Currently, BlunderChess has no check extension. The `gives_check` flag is already computed at `src/search/alpha_beta.rs:171` for LMR purposes, but is not used to adjust search depth.

## What to change

In `alpha_beta()`, when the side to move after `make_move` is in check (i.e., the move gives check), extend the recursive search depth by 1:

```rust
let gives_check = board.in_check();
let ext = if gives_check { 1 } else { 0 };
let new_depth = depth - 1 + ext;
```

The `gives_check` flag is already available (computed for LMR at line 171). Hoist it before the recursive calls and pass `new_depth` instead of `depth - 1`.

## Safety cap

Check-check-check chains can cause explosive depth extensions. Implement one of:

- **Option A (simplest):** Cap extensions globally — never let `ext > (depth / 2) + 1`. Track total extensions via a counter passed through the recursion, or compare `(root_depth - depth)` against a fraction of `root_depth`.
- **Option B (fractional):** Store extensions as a fractional accumulator (e.g., `ext_frac: i32` in the search state). Extend by 1 full ply only after accumulating enough fractional extensions. But this is overkill for a first cut.

Recommended: **Option A**, simple capped counter. Pass `extension_plied: u8` through the search state. Max total extensions = half the nominal remaining depth. If `extension_plied >= max_ext`, don't extend.

## Affected code

| File | Lines | Change |
|------|-------|--------|
| `src/search/alpha_beta.rs:162-187` | Recursive calls | Hoist `gives_check`, add extension logic |
| `src/search/state.rs` or `alpha_beta.rs` signal | Extension cap | Add `extension_plied: u8` or equivalent |

## Acceptance criteria

- [ ] When a move gives check, the search descends one ply deeper for that branch
- [ ] Check-check-check chains are capped — no infinite recursion or explosive depth
- [ ] `cargo test --lib --test benchmarks` passes
- [ ] No significant NPS regression at depth 8 (`cargo test --release --test benchmarks -- --ignored --nocapture`, compare `bench_nps_vs_depth` against `docs/performance.md` baseline)
- [ ] At least one integration test demonstrates the extension catching a tactic that would otherwise be missed at depth ~6

## Test position (candidate)

FEN: `r1b2rk1/ppp2ppp/2n2n2/1B2p1N1/1b1PP3/2N5/PPP2PPP/R1B1K2R b KQ - 4 8`

Black's Nc6-a5 attacks the bishop on b5. White's Ng5-f7+ is a check that wins the queen after Kxf7 Bxd7. Without check extensions, depth 6 may miss this because Nf7+ → Kxf7 is a quiet recapture at depth -1, and the bishop capture of the queen falls into QS. With check extensions, the search sees deeper into the forcing line.

## Out of scope

- Recapture extensions, pawn-to-7th extensions, singular extensions
- Fractional plies (Option B)
- Extending in quiescence search

## Comments
