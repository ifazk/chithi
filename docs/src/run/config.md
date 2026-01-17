# Configuration

<!-- Move this to a separate section when other tools are available -->

```toml
command = ["chithi", "sync", "-r", "--no-sync-snap", "--target-host=user@target"]
# Uncomment to disable the project.
#disabled = true

[run]
# Uncomment to add a randomized start delay for sequential tasks/parallel jobs
#max-initial-delay-secs = 10
# Uncomment to automatically restart jobs
#max-restarts = 5
# Uncomment to add delays be between restart jobs. In the example below, the
# first delay will be 2 mins, the second delay will be 4 mins, and the rest of
# restarts will delay 5 mins before starting.
#restart-delay-secs = [120,240,300]
# Uncomment to add a randomized jitter to restarts (in addition to the restart delay)
#max-restart-jitter = 10

[[task.backups.job]]
source= "tank/backups"
target= "onsite/backups"
```
