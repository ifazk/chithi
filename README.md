# Chithi (চিঠি)
OpenZFS replication tool.

# Port of syncoid to Rust
Chithi is a port of [syncoid](https://github.com/jimsalterjrs/sanoid) version
2.3 to Rust.

The `chithi sync` command has most features that are available syncoid, but the
command line interface is a little different. The feature differences are listed
below.

# Plugins
Chithi supports plugins via external commands. Running `chithi <subcommand>`
will look for a command named `chithi-<subcommand>` in your path and run try to
run that command, forwarding any arguments. The plugins `chithi-run`,
`chithi-status`, `chithi-cron`, and `chithi-sysd` are under development, and
thus you are discouraged from developing plugins with those names.

# Documentation
The documentation for using `chithi` for different use cases is currently under
development, but running `chithi help sync` is a good place to start if you are
looking for details.

# Chithi vs Syncoid 2.3

## Chithi features not found in syncoid 2.3
1. Cli `--{source,target}-host`. Can optionally set the source and target
   separately from the datasets.
2. Cli `--skip-optional-commands`. This can be used with `--no-command-checks`
   to control what commands get enabled.
3. When both the source and target are remote, we can run `pv` on the source
   machine over ssh.
4. Cli `--prune-formats`. Can use "--prune-format chithi --prune-format syncoid"
   to prune both formats. Defaults to "--prune-format chithi" if not set.
5. Cli `--dry-run`.
6. Plugins. You can run commands in your path of the form `chithi-<command>` by
   running `chithi <command>`.
7. Cli `--use-bookmarks`. This option will agressively fetch both snapshots and
   bookmarks for computing incremental sends. Useful for infrequent replication
   with aggressive snapshot pruning at source.
8. For `--create-bookmark`, by default we name them with
   `chithi_{identifier}{hostname}`. This can be changed to the syncoid 2.3 style
   with `--syncoid-bookmarks`.
9. We have `--max-bookmarks` to cleanup bookmarks after creating new bookmarks.
   This is off by default. There's very little reason to delete bookmarks since
   they are extermely cheap, but sometimes it is nice to tidy things up.
10. The `--preserve-properties` flag handles user properties that contain tab
    characters properly. The cost of doing this is (1) we make an additional
    call to just get the list of local properties, and (2) we fetch the each
    user property separately. An alternative approach would have been to use
    OpenZFS's json output, but the json output feature is too new and not widely
    available.
11. Cli `--timestamp-format`.

## Current feature deviations/shortcomings

1. Chithi: For hostname checks for `syncoid:sync`, the machine's hostname must
   be less than 255 characters long.
2. Chithi: We only support platforms which have the `-t` option for zfs, i.e. we
   don't reimpelment the fallback snapshot fetching in syncoid. This means no
   solaris.
3. Chithi: We use the regex-lite crate for rexeg, and therefore do not support
   unicode case insensitivity or unicode character classes like `\p{Letter}`.
4. Chithi: We do not support the insecure direct connection feature of syncoid.
5. Chithi: For recursive syncs, by default we do a recrursive recv check before
   we start. This is to prevent multiple instances of chiti syncs for the same
   source and target running at the same time. This can be turned off using the
   `--no-recv-check-start` flag.
6. When using bandwidth limits with a local send/recv, syncoid prefers to use
   the source bandwidth limit. We use the source bandwidth limit for
   limiting network transfers, so we ignore it completely for local send/recv.
   We interpret the target bandwidth limit for limiting disk writes, so we only
   use that for local send/recv.

# Contributing
I am not accepting PRs or contributions to the project. The project isn't ready
for contributions. The code here is GPLv3 through, so you may fork the project
under that license if you'd like to to take the project in a different
direction, or if the updates here are too slow.

# Name Meaning
Chithi (চিঠি) is the Bangla word for letter or mail.
