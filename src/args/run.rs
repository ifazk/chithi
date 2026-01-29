//  Chithi: OpenZFS replication tools
//  Copyright (C) 2025-2026  Ifaz Kabir

//  This program is free software: you can redistribute it and/or modify
//  it under the terms of the GNU General Public License as published by
//  the Free Software Foundation, either version 3 of the License, or
//  (at your option) any later version.

//  This program is distributed in the hope that it will be useful,
//  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//  GNU General Public License for more details.

//  You should have received a copy of the GNU General Public License
//  along with this program.  If not, see <https://www.gnu.org/licenses/>.

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

    /// Creates pid files in /var/run/chithi for sequential tasks and parallel
    /// jobs that are used to prevent multiple instances from running at the
    /// same time. This flag should only be used when running as root.
    #[arg(long)]
    pub create_pid_files: bool,

    /// Filters the jobs to run using tags
    #[arg(long)]
    pub tags: Option<String>,

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
