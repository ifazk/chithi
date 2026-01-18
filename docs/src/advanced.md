# Advanced sync

Chithi offers many advanced options for syncing. We highlight some of options
useful for common replication senarios, but a full set of options can be found
using the following command.

    chithi help sync

## Remote Hosts

The source and target may be specified as local dataset, a remote host with a
dataset in the form [user@]host:dataset. The following is an example of
replicating between two remote hosts.

    chithi sync user1@remotehost:sourcepool/myfiles user2@anotherhost:targetpool/myfiles
    
The remote hosts can be separately set using the --source-host and
--target-host options. The following command is equivalent to the above command.

    chithi sync --source-host=user1@remotehost --target-host=user2@anotherhost sourcepool/myfiles targetpool/myfiles

OpenZFS allows component names to have `:`, including in pool names. In the
following, Chithi treats `remotehost` and `anotherhost` as remote hosts, not as
part of the pool name.

    chithi sync remotehost:sourcepool/myfiles anotherhost:targetpool/myfiles

But if we pass empty source and target host options, then we can use local pools
that contain `:` in the pool name. In the following `prefix` is *not* treated as
a remote host.

    chithi sync --source-host= --target-host= prefix:sourcepool/myfiles prefix:targetpool/myfiles

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

## CLI Options

```
Replicates a dataset to another pool

Usage: chithi sync [OPTIONS] <SOURCE> <TARGET>

Arguments:
  <SOURCE>
  <TARGET>

Options:
      --compress <FORMAT>
          Compresses data during transfer. Currently accepted options are gzip, pigz-fast, pigz-slow, zstd-fast, zstdmt-fast, zstd-slow, zstdmt-slow, lz4, xz, lzo & none [default: lzo]
      --identifier <EXTRA>
          Extra identifier which is included in the snapshot name. Can be used for replicating to multiple targets
  -r, --recursive
          Also transfers child datasets
      --skip-parent
          Skips syncing of the parent dataset. Does nothing without '--recursive' option
      --source-bwlimit <SOURCE_BWLIMIT>
          Bandwidth limit in bytes/kbytes/etc per second on the source transfer
      --target-bwlimit <TARGET_BWLIMIT>
          Bandwidth limit in bytes/kbytes/etc per second on the target transfer
      --mbuffer-size <VALUE>
          Specify the mbuffer size, please refer to mbuffer(1) manual page [default: 16M]
      --pv-options <OPTIONS>
          Configure how pv displays the progress bar [default: "-p -t -e -r -b"]
      --no-stream
          Replicates using newest snapshot instead of intermediates
      --timestamp-format <TIMESTAMP_FORMAT>
          Timestamp format. All invalid characters in the format will be dropped. Formatting details can be found in the chrono::format::strftime documentation [default: %Y-%m-%d:%H:%M:%S-GMT%:z]
      --no-sync-snap
          Does not create new snapshot, only transfers existing
      --keep-sync-snap
          Does not prune sync snaps at the end of transfers
      --create-bookmark
          Creates a zfs bookmark for the newest snapshot on source after replication succeeds. Unless --syncoid-bookmarks is set, the bookmark name includes the identifier if set
      --syncoid-bookmarks
          Use the sanoid/syncoid 2.3 bookmark behaviour. This should be treated as an experinmental feature, and may not be kept in future minor revisions
      --syncoid-sync-check
          Use "syncoid:sync" property to check if we should sync sync. This should be treated as an experinmental feature, and may not be kept in future minor revisions
      --prune-format <SNAPFORMAT>
          If transfer creates new sync snaps, this option chooses what kind of snapshot formats to prune at the end of transfers. Current options are syncoid and chithi. Needs to be passed multiple times for multiple formats [default: chithi]
      --use-hold [<USE_HOLD>]
          Adds a hold to the newest snapshot on the source and target after replication and removes the hold after the next successful replication. The hold name includes the identifier if set. This allows for separate holds in case of multiple targets. Can be optionally passed the value "syncoid" to make syncoid compatible holds [default: false] [possible values: true, false, syncoid]
      --preserve-recordsize
          Preserves the recordsize on inital sends to the target
      --preserve-properties
          Preserves locally set dataset properties similar to the zfs send -p flag, but will also work for encrypted datasets in non raw sends. Properties are manually fetched on the source and manually written to on the target, with a blacklist of properties that cannot be written
      --no-rollback
          Does not rollback snapshots on target (it probably requires a readonly target)
      --delete-target-snapshots
          With this argument, snapshots which are missing on the source will be destroyed on the target. Use this if you only want to handle snapshots on the source
      --exclude-datasets <REGEX>
          Exclude specific datasets that match the given regular expression. Can be specified multiple times
      --exclude-snaps <REGEX>
          Exclude specific snapshots that match the given regular expression. Can be specified multiple times. If a snapshot matches both exclude-snaps and include-snaps patterns, then it will be excluded
      --include-snaps <REGEX>
          Only include snapshots that match the given regular expression. Can be specified multiple times. If a snapshot matches both exclude-snaps and include-snaps patterns, then it will be excluded
      --use-bookmarks [<USE_BOOKMARKS>]
          Use bookmarks for incremental syncing. When set to "always" (assumed if no value is passed), we fetch bookmarks as well as snapshots when computing incremental sends [default: fallback] [possible values: always, fallback]
      --max-bookmarks <MAX_BOOKMARKS>
          Prune bookmarks. Bookmarks are not
      --send-options <OPTIONS>
          Use advanced options for zfs send (the arguments are filtered as needed), e.g. chithi --send-options="Lc e" sets zfs send -L -c -e ... [default: ]
      --recv-options <OPTIONS>
          Use advanced options for zfs receive (the arguments are filtered as needed), e.g. chithi --recv-options="ux recordsize o compression=lz4" sets zfs receive -u -x recordsize -o compression=lz4 ... [default: ]
  -c, --ssh-cipher <CIPHER>
          Passes CIPHER to ssh to use a particular cipher set
  -P, --ssh-port <PORT>
          Connects to remote machines on a particular port
  -F, --ssh-config <FILE>
          Uses config FILE for connecting to remote machines over ssh
  -i, --ssh-identity <FILE>
          Uses identity FILE to connect to remote machines over ssh
  -o, --ssh-option <OPTION>
          Passes OPTION to ssh for remote usage. Can be specified multiple times
      --debug
          Prints out a lot of additional information during a chithi run. Logs overridden by --quiet and RUST_LOG environment variable
      --quiet
          Supresses non-error output and progress bars. Logs overridden by RUST_LOG environment variable
      --dump-snaps
          Dumps a list of snapshots during the run
      --no-command-checks
          Passes OPTION to ssh for remote usage. Can be specified multiple times
      --skip-optional-commands <SKIP_OPTIONAL_COMMANDS>
          A comma separated list of optional commands to skip. Current values are: sourcepv localpv targetpv compress localcompress sourcembuffer targetmbuffer localmbuffer [default: ]
      --dry-run
          Do a dry run, without modifying datasets and pools. The dry run functionality is provided on a best effort basis and may break between minor versions
      --no-resume
          Don't use the ZFS resume feature if available
      --no-clone-handling
          Don't try to recreate clones on target. Clone handling is done by deferring child datasets that are clones to a second pass of syncing, so this flag is not meaningful without the --recursive flag
      --no-privilege-elevation
          Bypass the root check, for use with ZFS permission delegation
      --source-host <SOURCE_HOST>
          Manually specifying source host (and user)
      --target-host <TARGET_HOST>
          Manually specifying target host (and user)
      --force-delete
          Remove target datasets recursively if there are no matching snapshots/bookmarks (also overwrites conflicting named snapshots)
      --no-recv-check-start
          Prevents the recursive recv check at the start of the sync
  -h, --help
          Print help
```
