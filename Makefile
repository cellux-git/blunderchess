.PHONY: linux windows arm clean test cross-check

CROSS := $(shell command -v cross 2>/dev/null)

linux:
	cargo build --release
	@echo "Done: target/release/blunderchess"

windows: cross-check
	@if docker ps >/dev/null 2>&1; then \
		cross build --release --target x86_64-pc-windows-gnu; \
	elif sg docker -c 'true' 2>/dev/null; then \
		sg docker -c 'cross build --release --target x86_64-pc-windows-gnu'; \
	else \
		echo "ERROR: Cannot access Docker. Run: newgrp docker"; exit 1; \
	fi
	@echo "Done: target/x86_64-pc-windows-gnu/release/blunderchess.exe"

arm: cross-check
	@if docker ps >/dev/null 2>&1; then \
		cross build --release --target aarch64-unknown-linux-gnu; \
	elif sg docker -c 'true' 2>/dev/null; then \
		sg docker -c 'cross build --release --target aarch64-unknown-linux-gnu'; \
	else \
		echo "ERROR: Cannot access Docker. Run: newgrp docker"; exit 1; \
	fi
	@echo "Done: target/aarch64-unknown-linux-gnu/release/blunderchess"

cross-check:
	@if [ -z "$(CROSS)" ]; then \
		echo "ERROR: 'cross' not found."; \
		echo "Install with: cargo install cross"; \
		echo "Requires Docker: sudo apt install docker.io"; \
		exit 1; \
	fi

clean:
	cargo clean

test:
	cargo test --lib --test benchmarks
