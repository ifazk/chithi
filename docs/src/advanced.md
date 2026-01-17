# Advanced sync

Chithi offers many advanced options for syncing. We highlight some of options
useful for common replication senarios, but a full set of options can be found
using the following command.

    chithi help sync

## External Snapshotting tools

Chithi is in many ways, expected to be used with external snapshotting tools,
such as [Sanoid](https://github.com/jimsalterjrs/sanoid/). In fact, the `sync`
command in Chithi is a port of the Syncoid tool that comes bundled with Sanoid.

Snapshotting tools like Sanoid automatically create and prune zfs snapshots of
datasets, and for many cases, the snapshotting tool is run on both the source
and target datasets.

## Sync snaps

Chithi by default creates what it calls a sync snap at the source dataset
whenever it replicates a dataset. This is to ensure that the latest version of
the files in the dataset are synced when the replication is started.

The sync snaps are named `chithi_{hostname}_{date}`, where `{hostname}` is the
hostname of the machine running the `chithi` command, and `{date}` is a
timestamp of when the replication was started.

### Timestamp format

The timestamp format can be changed using the `--timestamp-format` option.
Documentation of all the options in the timestamp format can be found in the
[chrono crate
documentation](https://docs.rs/chrono/0.4.43/chrono/format/strftime/).

    chithi sync --timestamp-format '%Y-%m-%d:%H:%M:%S-GMT%:z' sourcepool/myfiles targetpool/myfiles

### Extra identifier

An extra identifer can be passed to the `sync` command to change the sync snap names to
`chithi_{identifier}{hostname}_date`.

    chithi sync --identifer nightly sourcepool/myfiles targetpool/myfiles

### Sync snaps pruning

By default, chithi will also prune previous sync snaps (with the same hostname
and identifier) after replication. This pruning can be prevented using the
`--keep-sync-snap` flag.

    chithi sync --identifer nightly --keep-sync-snap sourcepool/myfiles targetpool/myfiles

### Preventing sync snaps

If there are rapid enough snapshots using an external snapshotting tool, you may
not need sync snaps. Sync snaps can be prevented using the `--no-sync-snap`
flag. For example, if there are hourly snapshots that are not pruned for over a 

    chithi sync --no-sync-snap sourcepool/myfiles targetpool/myfiles
