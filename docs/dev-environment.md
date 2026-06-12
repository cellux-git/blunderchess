# Development Environment

## Prerequisites

None of the following are installed on the current system. Install them before writing code.

### Rust toolchain

Install via [rustup](https://rustup.rs):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Restart your shell, then verify:

```sh
rustc --version   # should print 1.xx.x (latest stable)
cargo --version
```

### Optional: editor suggestions

- **VS Code** with `rust-analyzer` extension
- **Zed** (native Rust support)
- **Helix** (native Rust LSP)

No editor-specific configuration is required. `rust-analyzer` works out of the box in Cargo projects.

## Build

```sh
# Debug build (unoptimized, with debug symbols)
cargo build

# Release build (optimized)
cargo build --release

# Run (debug)
cargo run

# Run (release — use this for actual engine use)
cargo run --release
```

## Profile configuration

The default `dev` profile compiles quickly but runs ~10–50× slower. A custom profile bridges the gap:

```toml
# In Cargo.toml
[profile.engine-dev]
inherits = "dev"
opt-level = 2       # Optimize (like release)
debug = true        # Keep debug symbols
```

Use it with:

```sh
cargo run --profile engine-dev
cargo test --profile engine-dev
```

The standard `dev` profile remains available for fast compile-check iterations.

## Tests

```sh
# All tests
cargo test

# Tests with optimization (recommended for perft)
cargo test --profile engine-dev

# Single test
cargo test -- test_name

# Show output (for println!/eprintln! debugging)
cargo test -- --nocapture
```

## Running the engine

### Direct UCI mode (stdin/stdout)

```sh
cargo run --release
```

Then type UCI commands:

```
uci
isready
position startpos
go depth 5
quit
```

### With a UCI GUI (optional, post-v1)

Any UCI-compatible GUI works without configuration:

- **CuteChess** (cli + GUI): `cutechess-cli -engine cmd=./target/release/blunderchess -each tc=40/60`
- **Arena Chess GUI** (Windows)
- **Nibbler / BanksiaGUI** (cross-platform)

Point the GUI to the compiled binary and it handles UCI communication automatically.

## Project layout

```
blunderchess/
├── Cargo.toml
├── src/
│   ├── main.rs          # Entry point, slider table init, engine.run()
│   ├── lib.rs           # Module exports
│   ├── attack.rs        # Magic slider attack tables, knight/king/pawn attacks
│   ├── board.rs         # Board struct, make_move/unmake, FEN, draw detection
│   ├── book.rs          # Polyglot opening book (.bin reader, binary search)
│   ├── eval.rs          # Tapered evaluation, Eval struct, SEE
│   ├── movegen.rs       # Pseudo-legal + legal move generation, perft
│   ├── search.rs        # Alpha-beta, PVS, LMR, futility pruning, ID, Lazy SMP, MultiPV
│   ├── tt.rs            # Lock-free transposition table, huge pages
│   ├── types.rs         # Square, Piece, Color, Move (packed u16), CastlingRights
│   ├── uci.rs           # UCI command parsing, Engine, Ponder, book probe
│   └── zobrist.rs       # Zobrist key generation, incremental hash
├── tests/
│   └── benchmarks.rs    # Integration tests (tactical + performance benchmarks)
├── docs/
│   ├── dev-environment.md
│   ├── search-algorithm.md
│   └── adr/
├── CONTEXT.md
└── AGENTS.md
```

## Quick start after setup

```sh
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Build
cargo build --release

# 3. Test
cargo test

# 4. Run
echo "uci" | cargo run --release
```
