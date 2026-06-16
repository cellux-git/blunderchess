# 06 — Move geometry helpers to attack.rs

**Status:** `completed`

**Status**: `completed`
**Category**: `enhancement`

## Problem

Five general-purpose bitboard geometry functions live at the bottom of eval.rs:
- `file_mask(file) -> u64`
- `adjacent_files_mask(file) -> u64`
- `rank_mask_forward(sq, color) -> u64`
- `king_distance(a, b) -> u8`
- `attackers_to(board, sq, occ) -> u64`
- `smallest_attacker(board, sq, side, occ) -> Option<(Square, Piece)>`

These are pure bitboard/attack operations — no evaluation logic. They conceptually belong in the attack module alongside the magic bitboard functions. `attackers_to` and `smallest_attacker` in particular are attack-generation functions.

## What to change

1. Move all 6 functions from `src/eval.rs` to `src/attack.rs`.
2. Make them `pub` (currently they're module-level in eval.rs).
3. Update imports in eval.rs: `use crate::attack::{file_mask, ...}`.
4. No logic changes — just relocation.

## Key interfaces

- `attack.rs` gains 6 pub functions
- `eval.rs` imports them from `attack` instead of defining them locally
- `search.rs` / `uci.rs` / `board.rs` — no changes (they don't use these directly)
- `FILE_A` constant moves with `file_mask`

## Acceptance criteria

- [ ] All 6 functions live in `attack.rs`
- [ ] `eval.rs` imports them via `use crate::attack::*`
- [ ] All 86 unit tests pass
- [ ] All 10 tactical integration tests pass
- [ ] No compilation warnings
- [ ] No NPS regression (function relocation, zero runtime change)

## Out of scope

- Refactoring the helpers themselves
- Adding new geometry functions
- Moving other eval code

## Comments
