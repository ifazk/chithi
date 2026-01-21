## Expert Use Cases

## Calling from systemd

Systemd offers its own restarts, and job handling. When using these systemd
features, it is best to pass the `--no-run-config` flag to the runner. The
following is an example of systemd service and timer files using the flag.

```ini
[Unit]
Description=Chithi Run %i
Requires=local-fs.target
After=local-fs.target
StartLimitBurst=5
StartLimitIntervalSec=12h

[Service]
Type=simple
ExecStart=/usr/sbin/chithi run --no-run-config %i
Restart=on-failure
RestartSec=5min
```

The `RandomizedDelaySec` acts like the `max-initial-delay-secs` option.

```ini
[Unit]
Description=Cithi Run %i

[Timer]
OnCalendar=daily
RandomizedDelaySec=1h
Persistent=true
# Optional if timer file is chithi-run@%i.timer
Unit=chithi-run@%i.service

[Install]
WantedBy=timers.target
```

Save the above files as `chithi-run@.service` and `chithi-run@.timer` in
`/etc/systemd/system/` and run `sudo systemctl daemon-reload`. Then a daily
schedule can be added to everything in the default project by running the
following (requires [list command](../list/list.md)).

```
for i in $(chithi list); do sudo systemctl enable chithi-run@$i.timer --now; done
```

## Calling from cron

When calling the runner using cron, it may be necessary to stop multiple
instances of tasks and jobs from running at the same time. For root crontabs,
the `--create-pid-files` can be passed to the runner which creates lock files in
`/var/run/chithi`.

For user crontabs, the `flock` command needs to be used with the runner passing
in a user writable location for the lock file.

## Using with `timeout` command

The task runner does not support a timeout feature, so there may be a need to
use the `timeout` command in the runner. For example, the following is an
incorrect attempt to use `timeout` with `chithi sync`.

```toml
# This example is incorrect

[[task.home.job]]
command = ["timeout", "6h", "chithi", "sync", "-r", "--no-sync-snap", "--target-host=user@target"]
source = "tank/home/user1"
target = "onsite/home/user1"
```

The `timeout` command only terminates its own child process, so in the above
example `chithi sync` would be killed, but none of the children of `chithi sync`
would be terminated. The following runs chithi in a `sh` shell to get some of
the shell job control benefits where grand childen are interrupted when the
shell exits.

```toml
# This example is better

[[task.home.job]]
command = ["timeout", "6h", "sh", "-c", "chithi sync -r --no-sync-snap --target-host=user@target tank/home/user1 onsite/home/user1"]
```

However, timeout has wierd interactions with restarts. If `timeout 6h` is used with restarts.

### Will the runner ever get a timeout feature that works with restarts?

At least not a built-in timeout. There are a few architectural decisions at play.

1. The project does implement any signal handlers. We prefer to keep signal
   behaviour completely predictable.
2. The chithi project does not write platform specific code for Linux and
   FreeBSD.
3. Unix/POSIX does not have good ways of waiting on child programs to finish and
   on a timer at the same time (without using signal handlers).

There are things we could do, such as use the timeout command itself, and run
commands in a `sh` shell automatically, and keeping track of time elapsed, etc,
but at I'm currently unwilling to have the runner depend on an external program,
and I'm currently also unwilling to just implement a timeout command in chithi
just to not depend on an external program.
