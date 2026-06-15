# blunderchess

A UCI chess engine written in Rust.

## Build

### Linux (default)

```bash
make linux          # release build
# Or directly:
cargo build --release
```

Binary: `target/release/blunderchess`

### Windows (optional, via cross-compilation)

Uses [`cross`](https://github.com/cross-rs/cross) and Docker to cross-compile a native Windows `.exe` from Linux.

**Prerequisites (one-time):**

```bash
cargo install cross                      # cross-compilation tool
sudo apt install docker.io               # required by cross
sudo systemctl enable --now docker
sudo usermod -aG docker $USER            # log out/in after
```

**Build:**

```bash
make windows
```

If you get `permission denied` on the Docker socket, either log out/in
(to activate the `docker` group) or run:

```bash
sg docker -c "make windows"
```

Binary: `target/x86_64-pc-windows-gnu/release/blunderchess.exe`

### ARM64 / Android (optional, via cross-compilation)

Same prerequisites as Windows above.

```bash
make arm
```

Binary: `target/aarch64-unknown-linux-gnu/release/blunderchess`

Runs on ARM64 Linux (Raspberry Pi 4/5, AWS Graviton) and on Android via
[Termux](https://termux.dev) (Snapdragon Elite, MediaTek Dimensity, etc.).

### Dev build

```bash
cargo build                              # debug, faster compile
```

## Test

```bash
cargo test --lib                    # unit tests (fast)
cargo test --test benchmarks        # integration tests
cargo test --lib --test benchmarks  # all tests
```

## Run

Paste the binary path into your GUI (Arena, CuteChess, Nibbler) as a UCI engine.

Or pipe commands directly:

```bash
echo -e "position startpos\ngo depth 8\nquit" | target/release/blunderchess
```

## Key options

```
setoption name Hash value 64        # MB (default 64)
setoption name Threads value 4      # CPU threads
setoption name MultiPV value 3      # multi-PV analysis
setoption name OwnBook value true
setoption name BookFile value /path/to/book.bin
```
