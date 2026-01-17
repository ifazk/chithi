# Introduction

`chithi` is a command line tool to sync OpenZFS snapshots between local or
remote datasets. This is generally accomplished by passing appropriate options,
source, and target to the `chithi sync` command.

Beyond the `sync` command, there are additional commands and configuration
options available for running sync tasks by name.

## Rust Port of Syncoid

The `sync` command in chithi is a port of Syncoid 2.3 from the [sanoid
project](https://github.com/jimsalterjrs/sanoid). Chithi wouldn't have happend
without all of the work that went into Syncoid, so all the contributors to
Syncoid have my thanks!
