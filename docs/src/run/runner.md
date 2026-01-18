# `run` command

The `sync` command can get very verbose and error prone when manually running
the same sync task repeatedly. For these situations, we provide the runner
plugin, which allows running sync tasks by name.

It is configured by configuring the TOML file `/etc/chithi/chithi.toml`. An
example is shown below.

```toml
command = ["chithi", "sync", "-r", "--no-sync-snap", "--target-host=user@target"]

[run]
# Uncomment to automatically restart jobs on failure.
#max-restarts = 5

[[task.backups.job]]
source = "tank/backups"
target = "onsite/backups"

[task.home]
# Uncomment to make jobs in the task run in parallel.
#parallel = true
[[task.home.job]]
source = "tank/home/user1"
target = "onsite/home/user1"
[[task.home.job]]
source = "tank/home/user2"
target = "onsite/home/user2"
```

Then the `backups` task can be run using `chithi run backups`, no need to
remember the long sync command. A simple `chithi run` will both tasks `backups`
and `home` in parallel, but jobs in a task run sequentially unless the parallel
option is set to true.
