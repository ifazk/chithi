use log::error;
use serde::Deserialize;
use std::collections::HashMap;
use std::io;
use std::process::{Command, ExitStatus, Stdio};

#[derive(Deserialize)]
pub struct Job {
    pub command: Option<Vec<String>>,
    #[serde(default)]
    pub disabled: bool,
    pub source: String,
    pub target: String,
}

#[derive(Deserialize)]
pub struct Task {
    #[serde(rename = "command")]
    pub default_task_command: Option<Vec<String>>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub parallel: bool,
    #[serde(rename = "job")]
    pub jobs: Vec<Job>,
}

#[derive(Deserialize, Default)]
pub struct RunConfig {
    #[serde(rename = "max-initial-delay-secs")]
    pub max_inital_delay_secs: Option<u16>,
    #[serde(rename = "max-restarts")]
    pub max_restart_count: Option<u8>,
    #[serde(default, rename = "restart-delay-secs")]
    pub restart_delay_secs: Vec<u16>,
    #[serde(rename = "max-restart-jitter")]
    pub max_restart_jitter: Option<u16>,
}

impl RunConfig {
    pub fn restart_delay(&self, run_idx: usize) -> Option<u16> {
        let restart_delay = self
            .restart_delay_secs
            .get(run_idx)
            .or_else(|| self.restart_delay_secs.last())
            .copied();
        match (restart_delay, self.max_restart_jitter) {
            (Some(restart), Some(jitter)) => Some(restart + jitter),
            (x, y) => x.or(y),
        }
    }
}

#[derive(Deserialize)]
pub struct Project {
    #[serde(rename = "command")]
    pub default_project_command: Option<Vec<String>>,
    #[serde(default)]
    pub disabled: bool,
    pub run: Option<RunConfig>,
    #[serde(rename = "task")]
    pub tasks: HashMap<String, Task>,
}

pub struct NormalizedJob {
    pub command: Vec<String>,
    pub disabled: bool,
    pub source: String,
    pub target: String,
}

impl NormalizedJob {
    #[cfg(feature = "run")]
    pub fn get_command(&self) -> Command {
        let mut command = Command::new(&self.command[0]);
        command.args(&self.command[1..]);
        command.arg(&self.source);
        command.arg(&self.target);
        command
    }
    #[cfg(feature = "run")]
    pub fn run(&self) -> io::Result<ExitStatus> {
        self.get_command().stdin(Stdio::null()).status()
    }
}

pub struct NormalizedTask {
    pub disabled: bool,
    pub parallel: bool,
    pub jobs: Vec<NormalizedJob>,
}

pub struct NormalizedProject {
    pub disabled: bool,
    pub run_config: RunConfig,
    pub tasks: HashMap<String, NormalizedTask>,
}

impl Project {
    pub fn normalize(self, proj_name: &str) -> io::Result<NormalizedProject> {
        let proj_loc = Loc::new(proj_name);
        Self::check_command_maybe(&self.default_project_command, &proj_loc)?;
        let tasks: io::Result<HashMap<String, NormalizedTask>> = self.tasks.into_iter().map(|(task_name, task)| {
            let task_loc = proj_loc.extend_task(&task_name);
            Self::check_command_maybe(&task.default_task_command, &task_loc)?;
            let task_command = task
                .default_task_command
                .or_else(|| self.default_project_command.clone());
            let task_disabled = task.disabled || self.disabled;
            let jobs: Vec<_> = task.jobs.into_iter().enumerate().map(|(job_num, job)| {
                let job_loc = task_loc.extend_job(job_num);
                Self::check_command_maybe(&job.command, &job_loc)?;
                let job_command = job.command.or_else(|| task_command.clone());
                if let Some(command) = job_command {
                    Ok(NormalizedJob {
                        command,
                        disabled: job.disabled || task_disabled,
                        source: job.source,
                        target: job.target,
                    })
                } else {
                    error!("command not set for {job_loc}, please set a command at the job, task, or project level");
                    Err(io::Error::other(format!("command not set for task {job_loc}, please set a command at the job, task, or project level")))
                }
            }).collect::<Result<_,_>>()?;
            Ok((task_name, NormalizedTask {
                            disabled: task_disabled,
                            parallel: task.parallel,
                            jobs,
                        }))
        }).collect();
        tasks.map(|tasks| NormalizedProject {
            disabled: self.disabled,
            run_config: self.run.unwrap_or_default(),
            tasks,
        })
    }

    fn check_command_maybe(command: &Option<Vec<String>>, loc: &Loc) -> io::Result<()> {
        if let Some(command) = command {
            Self::check_command(command, loc)?
        }
        Ok(())
    }
    fn check_command(command: &[String], loc: &Loc) -> io::Result<()> {
        if command.is_empty() {
            error!(
                "invalid 0 length command for {loc}, please set a command with at least the command name"
            );
            return Err(io::Error::other(format!(
                "invalid 0 length command for {loc}, please set a command with at least the command name"
            )));
        };
        if command[0].as_str() == "chithi" && command.len() < 2 {
            error!(
                "invalid chithi command found with no args for {loc}, please set a sync subcommand"
            );
            return Err(io::Error::other(format!(
                "invalid chithi command found with no args for {loc}, please set a sync subcommand"
            )));
        }
        // This is a safeguard for beginners.
        // TODO decide if this is too much. It might be fun to do chithi run recursively.
        if command[0].as_str() == "chithi" && command[1].as_str() != "sync" {
            error!("invalid chithi command found for {loc}, please set a sync subcommand");
            return Err(io::Error::other(format!(
                "invalid chithi command found for {loc}, please set a sync subcommand"
            )));
        }
        Ok(())
    }
}

struct Loc<'a, 'b> {
    task_name: Option<&'a str>,
    job_num: Option<usize>,
    proj_name: &'b str,
}

impl<'a, 'b> Loc<'a, 'b> {
    fn new(proj_name: &'b str) -> Self {
        Self {
            task_name: None,
            job_num: None,
            proj_name,
        }
    }
    fn extend_task(&self, task_name: &'a str) -> Self {
        Self {
            task_name: Some(task_name),
            job_num: self.job_num,
            proj_name: self.proj_name,
        }
    }
    fn extend_job(&self, job_num: usize) -> Self {
        Self {
            task_name: self.task_name,
            job_num: Some(job_num),
            proj_name: self.proj_name,
        }
    }
}

impl<'a, 'b> std::fmt::Display for Loc<'a, 'b> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(task) = self.task_name {
            write!(f, "task {task} ")?
        };
        if let Some(job_num) = self.job_num {
            write!(f, "job {job_num} in ")?
        };
        write!(f, "project {}", self.proj_name)
    }
}
