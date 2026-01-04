.PHONY: check check_linux check_freebsd build build_linux build_freebsd test test_help

check: check_linux

check_linux:
	cargo fmt --check
	cargo check --target x86_64-unknown-linux-musl
	cargo clippy --target x86_64-unknown-linux-musl

check_freebsd:
	cargo fmt --check
	cargo check --target x86_64-unknown-freebsd
	cargo clippy --target x86_64-unknown-freebsd

build: build_linux build_freebsd

build_linux: check check_linux
	cargo build --release --target x86_64-unknown-linux-musl
	file target/x86_64-unknown-linux-musl/release/chithi
	ls -lah target/x86_64-unknown-linux-musl/release/chithi

build_freebsd: check_freebsd
	cargo build --release --target x86_64-unknown-freebsd
	file target/x86_64-unknown-freebsd/release/chithi
	ls -lah target/x86_64-unknown-freebsd/release/chithi

TEST_ARGS=

test: check
	cargo run --bin chithi -- ${TEST_ARGS}

test_help: check
	cargo run --bin chithi -- -h source target
