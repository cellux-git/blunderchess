# Mailbox board representation with bitboard attack detection

**Status**: accepted

## Context

A chess engine must represent the board (which pieces are on which squares) efficiently for move generation, evaluation, and search. Two mainstream choices: mailbox (array-based, `[Piece; 64]`) and bitboards (64-bit integers, one per piece type/color, where each bit is a square).

## Decision

Use mailbox as the primary representation, augmented with bitboard fields for O(1) attack detection. The board is `[Option<Piece>; 64]` with `pieces_bb[6]`, `colors_bb[2]`, and `occupancy` bitboard fields kept in sync on every make/unmake.

## Why

- Mailbox is the simplest representation to implement and debug. Move generation, make/unmake, and legality filtering are straightforward with array indexing.
- For a hobby engine where the primary goal is building a functional frame and learning Rust, simplicity and correctness beat speed.
- The architecture is structured so that board representation is an internal detail: `Move`, `Square`, `Piece`, and `Color` types are representation-agnostic. `movegen`, `eval`, and `search` interact with `Board` through functions that don't expose the internal representation.
- Bitboards were incrementally added after the mailbox was stable, providing O(1) `is_attacked_by` via magic bitboard tables without rewriting movegen from scratch.

## Considered options

| Option | Rejected because |
|--------|------------------|
| Bitboards from day 1 | Steeper learning curve. Magic bitboards for sliding pieces add complexity before the basic engine frame is built. |
| 0x88 mailbox variant | More complex bounds-checking for move generation. Standard 64-element array is simpler. |
| Hybrid (mailbox + incremental bitboards) | Adds complexity keeping two representations in sync — but this was the chosen path after the mailbox was stable and unit-tested. |

## Consequences

- All piece types now use bitboard move generation: knights/king/pawns via compile-time lookup tables; sliders (bishop/rook/queen) via magic bitboards with runtime-generated tables. Attack detection is O(1) across all piece types.
- The `Board` struct includes `pieces_bb[6]`, `colors_bb[2]`, and `occupancy` alongside the mailbox `squares` array and `piece_list` for mixed-access flexibility.
- Debugging is significantly easier: printing a board as an 8×8 ASCII grid is a 10-line function.
- Bishop magic tables are generated at runtime via trial-and-error (`find_magic()` with seeded LCG); the precomputed CPW magics had collisions on 40/64 bishop squares with our mask/shift implementation. Rook magics remain from CPW (all 64 verified collision-free via exhaustive test).
- Magic table initialization is lazy (via `std::sync::Once`) and called from `main()`.
