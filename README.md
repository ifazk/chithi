# Chithi
OpenZFS replication tool.

# Port of syncoid to Rust
Chithi is a port of [syncoid](https://github.com/jimsalterjrs/sanoid) version
2.3 to Rust.

The `chithi sync` command has most features that are available syncoid, but the
command line interface is a little different. The feature differences are listed
in the [in progress
documentation](https://github.com/ifazk/chithi/blob/main/docs/src/compatibility.md).

# Plugins
Chithi supports plugins via external commands. Running `chithi <subcommand>`
will look for a command named `chithi-<subcommand>` in your path and run try to
run that command, forwarding any arguments. The plugins `chithi-run`,
`chithi-status`, `chithi-cron`, and `chithi-sysd` are under development, and
thus you are discouraged from developing plugins with those names.

# Documentation
The documentation for using `chithi` for different use cases is currently under
development, but running `chithi help sync` is a good place to start if you are
looking for details. The work in progress documentation can be viewed in the
[docs folder](https://github.com/ifazk/chithi/blob/main/docs/src/SUMMARY.md).

# Contributing
I am not accepting PRs or contributions to the project. The project isn't ready
for contributions. The code here is GPLv3 through, so you may fork the project
under that license if you'd like to to take the project in a different
direction, or if the updates here are too slow.

# Name Meaning
Chithi (চিঠি) is the Bangla word for letter or mail.
