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

To install the base flavour of `chithi` using `cargo`, you should run `cargo
install chithi --no-default-features`.

To install additional subcommands, you also need to add the features for the
additional commands, choosing one of the bundle or bin. For example `cargo
install chithi --no-default-features --features run-bin` or `cargo install
chithi --no-default-features --features run-bundle`, the former will add
`chithi-run` as a separate binary (which can still be run with `chithi run`),
and the latter will just add the `chithi run` subcommand directly to the
`chithi` binary.

Some commands are only available bundled form. Currently, this is true of the
`list` command, available via the `list` feature which is included by default.

## Downloading binaries

Binaries are available at [https://github.com/ifazk/chithi/releases]. Binaries
are available for both Linux and FreeBSD, and should be put in /usr/sbin.

## Security of recursive calls in task runner

The `chithi run` and `chithi-run` commands call themselves recursively. So the
installation path (usually `/usr/sbin`) needs to be secured against unauthorized
modifications and the binary itself needs to have permissions set correctly. See
[here](https://vulners.com/securityvulns/SECURITYVULNS:DOC:22183) for an exmaple
of a privilege escalation vulnerability resulting from recursive calls.
