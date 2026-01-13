use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "run")]
#[command(version, about = "Task Runner for chithi", long_about = None)]
pub struct RunArgs {
    /// Runs a job or sequential task without any delays or restarts. Useful
    /// when testing or when the runner is called from a script that handles its
    /// own delays and restarts (e.g. systemd). This flag has no effect when
    /// running parallel tasks or projects.
    #[arg(long)]
    pub no_run_config: bool,

    /// Name of project. The runner will look for a .toml file with this name in /etc/chithi/
    #[arg(long, default_value = "chithi")]
    pub project: String,

    /// Name of sync task in project (NAME), or a specific job in a task
    /// (NAME.JOB). If no tasks are provided, the all tasks in the project are
    /// run. If just a task name NAME is provided, the runner wil run all jobs
    /// in that task. If both a task name and job index NAME.JOB is provided,
    /// the runner will only run that job. The runner will recursively call
    /// itself using the NAME.JOB format for running parallel tasks. Running a
    /// job is also useful for debugging/testing when combined with the
    /// --no-run-config flag.
    pub task_or_job: Option<String>,
}
