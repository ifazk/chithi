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
	rm -rf target/chithi-base-x86_64-unknown-linux-musl
	cargo build --quiet --release --target x86_64-unknown-linux-musl --features base
	cp -l target/x86_64-unknown-linux-musl/release/chithi target/chithi-base-x86_64-unknown-linux-musl
	rm -rf target/chithi-run-x86_64-unknown-linux-musl
	cargo build --quiet --release --target x86_64-unknown-linux-musl --features run-bin
	cp -l target/x86_64-unknown-linux-musl/release/chithi-run target/chithi-run-x86_64-unknown-linux-musl

build_freebsd: check_freebsd
	rm -rf target/chithi-base-x86_64-unknown-freebsd
	cargo build --quiet --release --target x86_64-unknown-freebsd --features base
	cp -l target/x86_64-unknown-freebsd/release/chithi target/chithi-base-x86_64-unknown-freebsd
	rm -rf target/chithi-run-x86_64-unknown-freebsd
	cargo build --quiet --release --target x86_64-unknown-freebsd --features run-bin
	cp -l target/x86_64-unknown-freebsd/release/chithi-run target/chithi-run-x86_64-unknown-freebsd

TEST_ARGS=

test: check
	cargo run --bin chithi -- ${TEST_ARGS}

test_help: check
	cargo run --bin chithi -- -h source target
