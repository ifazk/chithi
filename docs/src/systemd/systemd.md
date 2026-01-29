# chithi-systemd

We provide a simple script for scheduling daily runs with systemd. It can be run
using `chithi systemd`. It requires the `jq` command to be available for most
functionality.

## Installation

The script is not bundled as part of the `cargo install`. It needs to be
manually added to the path from the [Github Releases
Page](https://github.com/ifazk/chithi/releases) or [directly from the
repo](https://github.com/ifazk/chithi/raw/refs/heads/main/scripts/chithi-systemd).

## CLI options

```
Usage: /usr/sbin/chithi-systemd [-h|--help] [--project PROJECT] [--all | --failed | --state STATE] <COMMAND>

Commands:
  init                   Initialize systemd service and timer files
  destroy                Clear any existing timers and delete project service and timer files
  clear                  Clear any existing timers
  list                   List current timers for project
  reload                 Enable all timers for project
  start                  Start the systemd services (works without reload)
  status                 Show the status of the systemd services
  stop                   Stop any currently running services for project

Options:
  -h, --help             Show this help message
      --project PROJECT  Apply command to the project PROJECT [default: chithi]
      --all              Shows all services and timers for list and status commands [default]
      --failed           Shows services and timers that failed for list and status commands
      --state STATE      Filters the output of the list and status commands by state
```

## Basic workflow

Pass a `--project` option if needed for the following commands.

1. Create a chithi project file with some tasks and jobs, e.g.
   `/etc/chithi/chithi.toml`.
2. Run `sudo chithi systemd init`.
3. Run `sudo chithi systemd reload` to schedule daily runs for all jobs and
   timers.
4. Run `sudo chithi systemd list` to list all the current timers for the
   project.

Now would be a good time to run all the services and then check their status.

5. Run `sudo chithi systemd start` to start all the services for the project.
6. Run `sudo chithi systemd status` to see the status of services for the project.

The `status` and `list` commands can take `--failed | --state STATE` flags to
filter which services to show.

## Adding new jobs and tasks

After new jobs and tasks are added to the project toml files, `sudo chithi
systemd reload` needs to be run to schedule the jobs.

## Deleting jobs or tasks

We recommend using the `status` command to see if any services are active before
deleting them. The `chithi systemd stop` command may need to be used, or the
`stop` command for `systemctl` services may be needed. Then delete the job or
task from the project file and run `sudo chithi systemd reload` to remove
schedules for deleted jobs.

## Not intended for advanced scheduling

This script cannot handle advanced scheduling needs, e.g. hourly, weekly or
monthly scheduling. For those senarios, we recommend using
[tags](../run/config.md) and creating your own scripts using the
`chithi-systemd` script as a starting point.
