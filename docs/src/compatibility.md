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

TODO
