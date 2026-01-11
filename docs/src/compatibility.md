# Syncoid

The `sync` command in chithi is a port of `syncoid` 2.3 from the [sanoid
project](https://github.com/jimsalterjrs/sanoid).

The main feature that we do not have from `syncoid` 2.3 is making direct
insecure connections between machines using the `--insecure-direct-connection`
flag.

## New features

1. There is a `--dry-run` flag, which skips over the modification commands, but
   runs everything else, and keeps track of what the state should be if the
   modification commands succeeded. The `--dry-run` flag combined with the
   `--debug` flag can be used to see everything all the commands that `chithi
   sync` would run.
2. The `--use-bookmarks` flag/option makes `chithi sync` aggressively look at
   both bookmarks and snapshots, not just fallback to bookmarks if there are no
   matching snapshots. Makes syncing work with `--create-bookmark` and
   `--no-rollback` if your hourly snapshots get pruned. Inspired by
   https://github.com/jimsalterjrs/sanoid/issues/602
3. The `--max-bookmarks` option can be used to prune sync bookmarks.
4. The `--skip-optional-commands` option can be used to prevent `chithi sync`
   from using compression, `pv`, `mbuffer`, etc, that are not needed for
   syncing.

The last one needs some explanation. Sometimes `muffer` is best avoided even
when available, since it makes network traffic very spiky when the disks are
slow. `mbuffer` makes it hard to tell if syncing is broken, or there's a network
problem. Plus in`syncoid`, using `--no-command-checks` assumes `pv`, `mbuffer`,
are available, but you can tell `chithi sync` which optional commands are not
available when you use the `--no-command-checks`. 

## Syncoid compatibility

Although `chithi` is a port of `syncoid`, by default it does not prune
bookmarks and snapshots created by `syncoid`. Chithi offers some flags and
options of working with datasets that were previously managed by `syncoid`.
Below are flags offered by `chithi` to make it act like `syncoid`.

TODO

## Misc compatibility notes

TODO organize the notes below to the headers above

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
