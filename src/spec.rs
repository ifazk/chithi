use log::{error, info};
use serde::Deserialize;
use std::collections::HashMap;
use std::io;
use std::process::{Command, ExitStatus, Stdio};
use std::thread::sleep;
use std::time::Duration;

#[derive(Deserialize)]
pub struct Job {
    pub command: Option<Vec<String>>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
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
    #[cfg(feature = "run")]
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
    #[cfg(feature = "run")]
    pub fn inital_delay(&self, loc: Loc) {
        // Initial delay
        if let Some(delay) = self.max_inital_delay_secs {
            let secs = rand::random_range(0..delay);
            if secs > 0 {
                info!("delaying {loc} by {}", Seconds(secs));
                sleep(Duration::from_secs(secs as u64));
            }
        };
    }
}

/// Simple human readable time
pub struct Seconds(pub u16);

impl std::fmt::Display for Seconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut seconds = self.0;
        let hours = seconds / 3600;
        seconds %= 3600;
        let minutes = seconds / 60;
        seconds %= 60;
        if hours > 0 {
            if hours == 1 {
                write!(f, "{hours} hour")?
            } else {
                write!(f, "{hours} hours")?
            }
            if minutes > 0 || seconds > 0 {
                write!(f, " ")?
            }
        };
        if minutes > 0 {
            if minutes == 1 {
                write!(f, "{minutes} minute")?
            } else {
                write!(f, "{minutes} minutes")?
            }
            if seconds > 0 {
                write!(f, " ")?
            }
        };
        if seconds > 0 {
            if seconds == 1 {
                write!(f, "{seconds} second")?
            } else {
                write!(f, "{seconds} seconds")?
            }
        }
        if hours == 0 && minutes == 0 && seconds == 0 {
            write!(f, "0 seconds")?
        }
        Ok(())
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
    pub source: Option<String>,
    pub target: Option<String>,
}

impl NormalizedJob {
    #[cfg(feature = "run")]
    pub fn get_command(&self) -> Command {
        let mut command = Command::new(&self.command[0]);
        command.args(&self.command[1..]);
        if let Some(source) = self.source.as_deref() {
            command.arg(source);
        }
        if let Some(target) = self.target.as_deref() {
            command.arg(target);
        }
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

impl NormalizedTask {
    pub fn get_enabled_jobs<'proj>(
        &'proj self,
        task_loc: Loc<'proj, 'proj>,
    ) -> impl Iterator<Item = Loc<'proj, 'proj>> {
        self.jobs.iter().enumerate().filter_map(move |(idx, job)| {
            let job_loc = task_loc.extend_job(idx);
            if job.disabled {
                info!("{job_loc} is disabled");
                None
            } else {
                Some(job_loc)
            }
        })
    }
}

pub struct NormalizedProject {
    pub name: String,
    pub disabled: bool,
    pub run_config: RunConfig,
    pub tasks: HashMap<String, NormalizedTask>,
}

impl NormalizedProject {
    pub fn get_loc<'proj, 'a>(&'proj self) -> Loc<'a, 'proj> {
        Loc::new(self.name.as_str())
    }

    pub fn get_enabled_tasks_or_jobs<'proj>(
        &'proj self,
    ) -> impl Iterator<Item = Loc<'proj, 'proj>> {
        let proj_loc = self.get_loc();
        self.tasks
            .iter()
            .filter_map(move |(task_name, task)| {
                if task.disabled {
                    info!("{} is disabled", self.get_loc().extend_task(task_name));
                    None
                } else if task.parallel {
                    Some(
                        Box::new(task.get_enabled_jobs(proj_loc.extend_task(task_name)))
                            as Box<dyn Iterator<Item = Loc<'proj, 'proj>>>,
                    )
                } else {
                    Some(Box::new(std::iter::once(proj_loc.extend_task(task_name)))
                        as Box<dyn Iterator<Item = Loc<'proj, 'proj>>>)
                }
            })
            .flatten()
    }
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
                    Self::check_sync_job(&command, &job_loc, job.source.is_some() && job.target.is_some())?;
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
            name: proj_name.to_string(),
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
                "invalid chithi command found with no args for {loc}, please set a chithi subcommand"
            );
            return Err(io::Error::other(format!(
                "invalid chithi command found with no args for {loc}, please set a chithi subcommand"
            )));
        }
        Ok(())
    }
    fn check_sync_job(
        command: &[String],
        loc: &Loc,
        source_target_is_some: bool,
    ) -> io::Result<()> {
        // This is a safeguard for beginners.
        // TODO decide if this is too much. It might be fun to do chithi run recursively.
        if command[0].as_str() == "chithi"
            && command[1].as_str() == "sync"
            && !source_target_is_some
        {
            error!("chithi sync command found for {loc}, but job did not have source and target");
            Err(io::Error::other(format!(
                "chithi sync command found for {loc}, but job did not have source and target"
            )))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy)]
pub struct Loc<'a, 'b> {
    pub task_name: Option<&'a str>,
    pub job_num: Option<usize>,
    pub proj_name: &'b str,
}

impl<'a, 'b> Loc<'a, 'b> {
    pub fn new(proj_name: &'b str) -> Self {
        Self {
            task_name: None,
            job_num: None,
            proj_name,
        }
    }
    pub fn extend_task(&self, task_name: &'a str) -> Self {
        Self {
            task_name: Some(task_name),
            job_num: self.job_num,
            proj_name: self.proj_name,
        }
    }
    pub fn extend_job(&self, job_num: usize) -> Self {
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
            write!(f, "job {job_num} ")?
        };
        if self.task_name.is_some() || self.job_num.is_some() {
            write!(f, "in ")?
        }
        write!(f, "project {}", self.proj_name)
    }
}
