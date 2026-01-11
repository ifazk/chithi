# Installation

There are two flavours of chithi, `chithi-base` which only comes with the
`chithi sync` command and `chithi-full` which comes the rest of the.

`chithi-base`, only comes with the `chithi sync` command. The rest of the
commands can be installed separately.

## Using cargo

Cargo is used to install rust development tools, and is usually installed using
the [rustup installer](https://rustup.rs/) for Rust, or from a distribution's
package repository.

To install the full flavour of `chithi` using `cargo`, you can just run `cargo
install chithi`.

To install the base flavour of `chithi` using `cargo`, you should run `cargo install
chithi --features base`.

To install additional subcommands, you also need to add the features for the
additional commands, choosing one of the bundle or bin. For example `cargo
install chithi --features base,run-bin` or `cargo install chithi --features
base,run-bundle`, the former will add `chithi-run` as a separate binary (which
can still be run with `chithi run`), and the latter will just add the `chithi
run` subcommand directly to the `chithi` binary.

## Downloading binaries

Binaries are available at https://github.com/ifazk/chithi/releases. Binaries are
available for both linux and freebsd, and should be put in /usr/bin.
