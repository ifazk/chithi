# Basic sync

The basic command in `chithi` is the `sync` command.

Suppose you have two zfs pools, `sourcepool` and `targetpool`, and a
dataset/filesystem `sourcepool/myfiles` that you want to replicate from
`sourcepool` to `targetpool`. Using the following command.

    chithi sync sourcepool/myfiles targetpool/myfiles

This will replicate all the data and snapshots from `sourcepool/myfiles` to (a
new dataset) `targetpool/myfiles`. Once the replication finishes and there are
updates to `sourcepool/myfiles`, the command can be run again and Chithi will
incrementally update the data in `targetpool/myfiles`.

By default chithi assumes that zfs commands can be run as root using the `sudo`
command. For non-root zfs delegation setups, the `--no-privilege-elevation` flag
can be passed to `chithi sync` to prevent `chithi` from using `sudo`.

## Dry run

The `sync` command can be passed a `--dry-run` flag, which makes Chithi skip
over the command that would modify datasets but assume that they ran
successfully.

    chithi sync --dry-run sourcepool/myfiles targetpool/myfiles

The `--dry-run` flag is useful to run with the `--debug` flag to see all the
commands that `chithi` would run without modifying the datasets.

## Remote syncing

The source or target can be in a remote system that is accessible via `ssh`. For
example, the following will replicate `myfiles` to a target pool using the ssh
user `user@remotehost`.

    chithi sync sourcepool/myfiles user@remotehost:targetpool/myfiles
    
And the following will replicate from a remote source pool to a local target
pool.

    chithi sync user@remotehost:sourcepool/myfiles targetpool/myfiles
