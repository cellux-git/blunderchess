# Zero external dependencies

**Status**: accepted

## Context

Rust has a rich crate ecosystem. Common chess engine dependencies include `rand` (Zobrist seed), `arrayvec` (stack-allocated move lists), `shakmaty` (chess library), and various CLI/parser crates.

## Decision

The engine uses **zero external crate dependencies in the core**. The only allowed exceptions are `log` + `env_logger` for structured diagnostics, which are tiny, widely used, and don't touch engine logic.

## Why

- All required functionality is available in `std`: `Arc`, `AtomicBool`, `HashMap`, `thread::spawn`, `Vec`, array indexing.
- Zobrist keys are generated at compile time with a `const fn` LCG, avoiding the `rand` dependency tree (which is large for what amounts to a one-time init).
- FEN parsing is a fixed-format, limited-complexity grammar best handled with simple string operations — parser libraries are overkill.
- Zero core deps means zero dependency conflicts, zero supply-chain risk, and instant builds. The compile-test cycle stays fast.
- For a hobby/learning project, implementing everything from scratch is part of the value. Relying on a chess library would defeat the purpose.

## Considered options

| Option | Rejected because |
|--------|------------------|
| `rand` for Zobrist | Compile-time LCG is 20 lines. `rand` pulls in a large dependency tree for a one-time startup task. |
| `pest`/`nom` for FEN parsing | FEN is trivial: split on `/`, map characters to pieces. Parser combinators add build time and cognitive overhead with no benefit. |
| `shakmaty` for board/types | This is a chess engine *building* project. Using a chess library removes the core learning and control. |
| `clap` for CLI args | The engine speaks UCI over stdin. There are no CLI arguments beyond maybe `--version`. `std::env::args` suffices. |
| `thiserror`/`anyhow` for errors | Error variants are simple enums with `Display` impls. External error crates add deps for marginal syntax sugar. |

## Exception: `log` + `env_logger`

These crates provide structured, level-gated logging to stderr. They are:
- Tiny (no dependency tree beyond `log` itself)
- The Rust ecosystem standard (familiar to any Rust developer)
- Critically useful for debugging search behavior (enable with `RUST_LOG=blunderchess=debug`)
- Completely excluded from the engine binary in release builds if desired (compile out via feature flag)

## Consequences

- All data structures (TT, board, move list) use standard library collections. The TT uses `Vec` with manual power-of-two indexing rather than a specialized hash table, which is standard for chess engines anyway.
- Adding a dependency later requires justifying it against this ADR.
