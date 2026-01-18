# Configuration

<!-- Much of this will be moved to a separate project section when other chithi commands are available -->

The task runner is configured using `.toml` files in `/etc/chithi/`, with the
default configuration file being `/etc/chithi/chithi.toml`.

```toml
# A project file consists of tasks.
command = ["chithi", "sync", "-r", "--no-sync-snap", "--target-host=user@target"]

[task.home]
# A task consists of jobs.
[[task.home.job]]
# This is a job.
source = "tank/home/user1"
target = "onsite/home/user1"
```

## Project files

The toml files in `/etc/chithi` are called project files. The only thing special
about `/etc/chithi/chithi.toml` file is that it is the default. This file does
not need to exist to run tasks or jobs in other project files.

If there is a project file `/etc/chithi/mysyncs.toml`, then `chith run --project
mysyncs` can be used to run the tasks in that file.

## Command overrides

Commands can be set at the project, task, or job levels with inner level
commands overriding commands set at the outer levels.

Each of the levels are by themselves optional, but each job in a project must be
assigned a command directly or via inheritence.

```toml
# If a top level command is set, it is inherited by all jobs in the project.
command = ["chithi", "sync", "-r", "--no-sync-snap", "--target-host=user@target"]

# Jobs in the backups task all inherit the top level command since they don't
# have commands assigned at the task level or job level.
[[task.backups.job]]
source = "tank/backups"
target = "onsite/backups"
[[task.backups.job]]
source = "tank/backups2"
target = "onsite/backups2"

[task.home]
# This is a command set at the task level, jobs in this task inherits the task
# command and not the project command.
command = ["chithi", "sync", "--no-sync-snap", "--target-host=user@target"]
[[task.home.job]]
source = "tank/home/user1"
target = "onsite/home/user1"
[[task.home.job]]
# This is a command set at the job level. This job uses the command set here.
command = ["chithi", "sync", "--keep-sync-snap"]
source = "tank/home/user2"
target = "user@target:onsite/home/user2"
[[task.home.job]]
# The command set in the above job does not carry over to this job. This job
# uses the task command.
source = "tank/home/user3"
target = "onsite/home/user3"
```

## Commands, sources and targets

The commands shown so far are all `chithi sync` commands, and jobs have both
`source` and `targets`. This is not required, any command can be used with the
task runner, including `syncoid` and `rsync`. The source and target fields are
just extra arguments added when running the command.

However, the runner does enforce one thing. If the command for a job is a
`chithi sync` command, then that job must have the source and target fields set.

```toml
[[task.home.job]]
# This is a valid command.
command = ["echo", "hello"]
[[task.home.job]]
# So is this, and it run the same command
target = "hello"
command = ["echo"]

# You can chain different commands.
[[task.rsync.job]]
source = "/root/sourcedir/"
target = "/zroot/target/"
command = ["rsync"]
[[task.rsync.job]]
command = ["zfs", "snapshot", "zroot/target@rsyncdone"]
[[task.rsync.job]]
command = ["curl", "https://healthcheck.lan"]
```

```toml
# This example is invalid since it is missing a target field.
[[task.home.job]]
source = "tank/home/user1"
command = ["chithi", "sync", "-r", "--no-sync-snap", "--target-host=user@target"]
```

## Restarts and Delays

A project file can have a section called `run` that describes restart behaviour
and randomized delays. The comments in the example below describe the available
options.

```toml
command = ["chithi", "sync", "-r", "--no-sync-snap", "--target-host=user@target"]

# The runner configuration section is optional, and fields in the runner
# configuration are also optional.
[run]
# Uncomment to add a randomized start delay for sequential tasks/parallel jobs.
#max-initial-delay-secs = 10
# Uncomment to automatically restart jobs.
#max-restarts = 5
# Uncomment to add delays be between restart jobs. In the example below, the
# first delay will be 2 mins, the second delay will be 4 mins, and the rest of
# restarts will delay 5 mins before starting.
#restart-delay-secs = [120,240,300]
# Uncomment to add a randomized jitter to restarts (in addition to the restart delay).
#max-restart-jitter = 10

[task.home]
[[task.home.job]]
source = "tank/home/user1"
target = "onsite/home/user1"
[[task.home.job]]
source = "tank/home/user2"
target = "onsite/home/user2"
```

## Disabling projects, tasks, and jobs

Projects, tasks, and jobs can be disabled. If an outer level is disabled, then
the inner level is also interpreted as disabled even if the inner level has an
explicit `disabled = false`.

```toml
command = ["chithi", "sync", "-r", "--no-sync-snap", "--target-host=user@target"]
# Uncomment to disable the project.
#disabled = true

[task.home]
# Uncomment to disable the task.
#disabled = true
[[task.home.job]]
# Uncomment to disable the job.
#disabled = true
source = "tank/home/user1"
target = "onsite/home/user1"
[[task.home.job]]
# Uncomment to disable the job.
#disabled = true
source = "tank/home/user2"
target = "onsite/home/user2"
```
