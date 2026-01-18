# Using with timeout command

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
would be terminated.

```toml
# This example is better

[[task.home.job]]
command = ["timeout", "6h", "sh", "-c", "chithi sync -r --no-sync-snap --target-host=user@target tank/home/user1 onsite/home/user1"]
```
