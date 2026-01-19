.PHONY: check check_linux check_freebsd build build_linux build_freebsd test test_help

CHITHI_BASE=--no-default-features
CHITHI_RUN=--no-default-features --features run-bin --bin chithi-run

check: check_linux

check_linux:
	cargo fmt --check
	cargo check --target x86_64-unknown-linux-musl
	cargo check --target x86_64-unknown-linux-musl ${CHITHI_BASE}
	cargo check --target x86_64-unknown-linux-musl ${CHITHI_RUN}
	@cargo check --target x86_64-unknown-linux-musl ${CHITHI_BASE} --features run-bin
	@cargo check --target x86_64-unknown-linux-musl ${CHITHI_BASE} --features run-bundle
	@cargo check --target x86_64-unknown-linux-musl ${CHITHI_BASE} --features list
	@cargo check --target x86_64-unknown-linux-musl ${CHITHI_BASE} --features run-bin,run-bundle,list
	cargo clippy --target x86_64-unknown-linux-musl
	cargo clippy --target x86_64-unknown-linux-musl ${CHITHI_BASE}
	cargo clippy --target x86_64-unknown-linux-musl ${CHITHI_RUN}
	@cargo clippy --target x86_64-unknown-linux-musl ${CHITHI_BASE} --features run-bin
	@cargo clippy --target x86_64-unknown-linux-musl ${CHITHI_BASE} --features run-bundle
	@cargo clippy --target x86_64-unknown-linux-musl ${CHITHI_BASE} --features list
	@cargo clippy --target x86_64-unknown-linux-musl ${CHITHI_BASE} --features run-bin,run-bundle,list

check_freebsd:
	cargo fmt --check
	cargo check --target x86_64-unknown-freebsd
	cargo clippy --target x86_64-unknown-freebsd

build: build_linux build_freebsd
	ls -lhi --color=auto target/chithi-*-linux-musl
	ls -lhi --color=auto target/chithi-*-freebsd
	file target/chithi-*-linux-musl
	file target/chithi-*-freebsd

build_linux: check check_linux
	@rm -f target/chithi-x86_64-unknown-linux-musl
	cargo build --quiet --release --target x86_64-unknown-linux-musl
	cp -l target/x86_64-unknown-linux-musl/release/chithi target/chithi-x86_64-unknown-linux-musl
	@rm -f target/chithi-base-x86_64-unknown-linux-musl
	cargo build --quiet --release --target x86_64-unknown-linux-musl ${CHITHI_BASE}
	cp -l target/x86_64-unknown-linux-musl/release/chithi target/chithi-base-x86_64-unknown-linux-musl
	@rm -f target/chithi-run-x86_64-unknown-linux-musl
	cargo build --quiet --release --target x86_64-unknown-linux-musl ${CHITHI_RUN}
	cp -l target/x86_64-unknown-linux-musl/release/chithi-run target/chithi-run-x86_64-unknown-linux-musl

build_freebsd: check_freebsd
	@rm -f target/chithi-x86_64-unknown-freebsd
	cargo build --quiet --release --target x86_64-unknown-freebsd
	cp -l target/x86_64-unknown-freebsd/release/chithi target/chithi-x86_64-unknown-freebsd
	@rm -f target/chithi-base-x86_64-unknown-freebsd
	cargo build --quiet --release --target x86_64-unknown-freebsd ${CHITHI_BASE}
	cp -l target/x86_64-unknown-freebsd/release/chithi target/chithi-base-x86_64-unknown-freebsd
	@rm -f target/chithi-run-x86_64-unknown-freebsd
	cargo build --quiet --release --target x86_64-unknown-freebsd ${CHITHI_RUN}
	cp -l target/x86_64-unknown-freebsd/release/chithi-run target/chithi-run-x86_64-unknown-freebsd

TEST_ARGS=

test: check
	cargo run --bin chithi -- ${TEST_ARGS}

test_help: check
	cargo run --bin chithi -- -h source target
