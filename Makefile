.PHONY: all build release test check fmt clippy install clean doc probe

all: build

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets -- -D warnings

check: fmt clippy test

install:
	cargo install --path . --force

install-release:
	./scripts/install.sh --release

clean:
	cargo clean

doc:
	cargo doc --no-deps

probe:
	./target/debug/razochar6e probe || cargo run -- probe

doctor:
	./target/debug/razochar6e doctor || cargo run -- doctor

completions:
	mkdir -p completions
	cargo run --quiet -- completions bash > completions/razochar6e.bash
	cargo run --quiet -- completions zsh > completions/_razochar6e
	cargo run --quiet -- completions fish > completions/razochar6e.fish
