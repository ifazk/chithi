# list command

When project files get large, it is useful to list all the tasks and jobs, and
perhaps iterate through them in a script. For this purpose, we offer the `list`
command.

```
Lists tasks and jobs in a chithi project

Usage: chithi list [OPTIONS] [TASK]

Arguments:
  [TASK]  Name of sync task in project (NAME). If no tasks are provided, the sequential tasks and jobs in parallel tasks will be listed

Options:
  -l, --long               Use a long listing format. Shows disabled, sources, targets, commands
  -H, --no-headers         Scripted mode for long listing. Has no effect if long listing is not enabled
      --skip-disabled      Skip disabled tasks and jobs in listings
      --tags <TAGS>        Filter by tags. Multiple tags can be included by seprating them with commas
      --project <PROJECT>  Name of project. Chithi will look for a .toml file with this name in /etc/chithi/ [default: chithi]
  -h, --help               Print help
  -V, --version            Print version
```
