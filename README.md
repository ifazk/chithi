# Chobi (ছবি) and Chithi (চিঠি)
Ports of Sanoid and Syncoid to Rust. Current focus is on Chithi (Syncoid).

# Not bug-for-bug compatible
The plan right now is to be as compatible as possible with syncoid, but I will
port features in a way that makes more sense in Rust. Particularly, Perl makes
it really easy to use regexes for things which would be very unidiomatic in
Rust. The functionality should all be there though, you should be able to to do
the same things but the command line interface might be a little different, and
some more escaping might be needed.

## Current feature deviations/shortcomings

1. Chithi: For hostname checks for `syncoid:sync`, the machine's hostname must
   be less than 255 characters long.
2. Chithi: We only support platforms which have the `-t` option for zfs, i.e. we
   don't reimpelment the fallback snapshot fetching in syncoid. This means no
   solaris.
3. Chithi: We use the regex-lite crate for rexeg, and therefore do not support
   unicode case insensitivity or unicode character classes like `\p{Letter}`.
4. Chithi: Not supporting insecure direct connection.
5. Chithi: For recursive syncs, by default we do a recrursive recv check before
   we start. This is to prevent multiple instances of chiti syncs for the same
   source and target running at the same time. This can be turned off using the
   `--no-recv-check-start` flag.
6. When using bandwidth limits with a local send/recv, syncoid prefers to use
   the source bandwidth limit. We use the source bandwidth limit for
   limiting network transfers, so we ignore it completely for local send/recv.
   We interpret the target bandwidth limit for limiting disk writes, so we only
   use that for local send/recv.

## Chithi features not found in syncoid 2.3
1. Cli `--{source,target}-host`.
2. Cli `--skip-optional-commands`. This can be used with `--no-command-checks`
   to control what commands get enabled.
3. When both the source and target are remote, we can run `pv` on the source
   machine over ssh.
4. Cli `--prune-formats`. Can use "--prune-format chithi --prune-format syncoid"
   to prune both formats. Defaults to "--prune-format chithi" if not set.
5. Cli `--dry-run`.
6. Plugins.
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

# Why Rust? Why Not Go?
There are no technical or social reasons why I'm choosing Rust. Go would have
been a better option, which I also have some experience with. But I just happen
to be mainly using Rust right now, and so things will be quicker to implement on
my end.

# Development note
It is an explicit goal to be single threaded, and use non-blocking code without
busy spinning. There should not be any thread spawning code anywhere here,
except in the case of parallel sends but even in that case individual sends
should be single threaded. It is also an explicit goal to only rely on posix
instead of needing separate special case code for Linux and FreeBSD.

# ETA when?
Perhaps never. If I get to understand all the features of sanoid and syncoid
through this project, that's more than enough for me. That being said I will
release the binary for chithi if I finish the following features.

## Current TODOs for Chithi

- Preserve properties

# Contributing
I am not accepting PRs or contributions to the project. The project isn't ready
for contributions. The code here is GPLv3 through, so you may fork the project
under that license if you'd like to to take the project in a different
direction, or if the updates here are too slow.

# Reporting issues
This project is not accepting any issues. I plan on opening up issues once
enough functionality is implemented.
