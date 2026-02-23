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

use log::error;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::PathBuf;

#[cfg(any(feature = "run-bin", feature = "run-bundle", feature = "list"))]
use crate::args::tags::TagFilter;
#[cfg(any(feature = "run-bin", feature = "run-bundle"))]
use log::info;
#[cfg(any(feature = "run-bin", feature = "run-bundle"))]
use std::process::{Command, ExitStatus, Stdio};

#[derive(Deserialize)]
pub struct Job {
    pub command: Option<Vec<String>>,
    #[serde(rename = "on-success")]
    pub on_success: Option<Vec<String>>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
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
    #[serde(rename = "on-success")]
    pub on_success: Option<Vec<String>>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Deserialize, Default)]
pub struct RunConfig {
    #[serde(rename = "max-initial-delay-secs")]
    pub max_initial_delay_secs: Option<u16>,
    #[serde(rename = "max-restarts")]
    pub max_restart_count: Option<u8>,
    #[serde(default, rename = "restart-delay-secs")]
    pub restart_delay_secs: Vec<u16>,
    #[serde(rename = "max-restart-jitter")]
    pub max_restart_jitter: Option<u16>,
}

impl RunConfig {
    #[cfg(any(feature = "run-bin", feature = "run-bundle"))]
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
    #[cfg(any(feature = "run-bin", feature = "run-bundle"))]
    pub fn initial_delay(&self, loc: Loc) {
        // Initial delay
        if let Some(delay) = self.max_initial_delay_secs {
            let secs = rand::random_range(0..delay);
            if secs > 0 {
                info!("delaying {loc} by {}", Seconds(secs));
                std::thread::sleep(std::time::Duration::from_secs(secs as u64));
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
    pub on_success: Option<Vec<String>>,
    pub disabled: bool,
    pub source: Option<String>,
    pub target: Option<String>,
    pub tags: HashSet<String>,
}

impl NormalizedJob {
    #[cfg(any(feature = "run-bin", feature = "run-bundle"))]
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
    #[cfg(any(feature = "run-bin", feature = "run-bundle"))]
    pub fn run(&self) -> io::Result<ExitStatus> {
        self.get_command().stdin(Stdio::null()).status()
    }
    #[cfg(any(feature = "run-bin", feature = "run-bundle"))]
    pub fn run_on_success(&self) {
        if let Some(on_success) = &self.on_success {
            let mut command = Command::new(&on_success[0]);
            command.args(&on_success[1..]);
            if let Err(e) = command.stdin(Stdio::null()).status() {
                log::warn!("running on-success command failed with {e}");
            }
        }
    }
    #[cfg(any(feature = "run-bin", feature = "run-bundle", feature = "list"))]
    pub fn doesnt_match(&self, tags: Option<&TagFilter>) -> bool {
        tags.as_ref().is_some_and(|tags| !tags.matches(&self.tags))
    }
}

pub struct NormalizedTask {
    pub disabled: bool,
    pub parallel: bool,
    pub on_success: Option<Vec<String>>,
    pub jobs: Vec<NormalizedJob>,
    pub tags: HashSet<String>,
}

impl NormalizedTask {
    #[cfg(any(feature = "run-bin", feature = "run-bundle"))]
    pub fn get_enabled_jobs<'proj>(
        &'proj self,
        task_loc: Loc<'proj, 'proj>,
        tags: Option<&TagFilter>,
    ) -> impl Iterator<Item = Loc<'proj, 'proj>> {
        self.jobs.iter().enumerate().filter_map(move |(idx, job)| {
            let job_loc = task_loc.extend_job(idx);
            if job.doesnt_match(tags) {
                None
            } else if job.disabled {
                info!("{job_loc} is disabled");
                None
            } else {
                Some(job_loc)
            }
        })
    }
    #[cfg(any(feature = "run-bin", feature = "run-bundle", feature = "list"))]
    pub fn doesnt_match(&self, tags: Option<&TagFilter>) -> bool {
        tags.as_ref().is_some_and(|tags| !tags.matches(&self.tags))
    }
    #[cfg(any(feature = "run-bin", feature = "run-bundle"))]
    pub fn spawn_on_success(&self) -> Option<std::process::Child> {
        if let Some(on_success) = &self.on_success {
            let mut command = Command::new(&on_success[0]);
            command.args(&on_success[1..]);
            command.stdin(Stdio::null());
            match command.spawn() {
                Ok(handle) => Some(handle),
                Err(e) => {
                    log::warn!("on success command failed with {e}");
                    None
                }
            }
        } else {
            None
        }
    }
}

/// An abstraction over tasks and jobs so that we can iterate over both
/// sequential tasks and jobs in parallel tasks
pub enum TaskOrJob<J, T> {
    Job(J),
    Task(T),
}

pub struct TaskOrJobIter<J, I> {
    current: Option<J>,
    rest: I,
}

impl<J, T, I: Iterator<Item = TaskOrJob<J, T>>> TaskOrJobIter<J, I> {
    pub fn new(iter: I) -> Self {
        Self {
            current: None,
            rest: iter,
        }
    }
}

impl<S, J: Iterator<Item = S>, I: Iterator<Item = TaskOrJob<J, S>>> Iterator
    for TaskOrJobIter<J, I>
{
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let from_current = self.current.as_mut().and_then(|x| x.next());
            if from_current.is_some() {
                return from_current;
            };
            self.current = None;
            let from_rest = self.rest.next();
            match from_rest {
                Some(TaskOrJob::Task(s)) => return Some(s),
                Some(TaskOrJob::Job(iter)) => {
                    // foo
                    self.current = Some(iter);
                    continue;
                }
                None => {
                    // iterator ended
                    return None;
                }
            }
        }
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

    #[cfg(any(feature = "run-bin", feature = "run-bundle"))]
    pub fn get_enabled_tasks_or_jobs<'proj>(
        &'proj self,
        tags: Option<&TagFilter>,
    ) -> impl Iterator<Item = Loc<'proj, 'proj>> {
        let proj_loc = self.get_loc();
        let iter = self.tasks.iter().filter_map(move |(task_name, task)| {
            if task.disabled {
                if task.jobs.iter().any(|j| !j.doesnt_match(tags)) {
                    // there is a job that wouldn't get filtered out
                    info!("{} is disabled", self.get_loc().extend_task(task_name));
                }
                None
            } else if task.parallel {
                Some(TaskOrJob::Job(
                    task.get_enabled_jobs(proj_loc.extend_task(task_name), tags),
                ))
            } else if task.doesnt_match(tags) {
                None
            } else {
                Some(TaskOrJob::Task(proj_loc.extend_task(task_name)))
            }
        });
        TaskOrJobIter::new(iter)
    }

    #[cfg(any(feature = "run-bin", feature = "run-bundle", feature = "list"))]
    pub fn list_independents(
        &self,
        skip_disabled: bool,
        tags: Option<&TagFilter>,
    ) -> impl Iterator<
        Item = TaskOrJob<
            impl Iterator<Item = (Loc<'_, '_>, &NormalizedJob)>,
            (Loc<'_, '_>, &NormalizedTask),
        >,
    > {
        let proj_loc = self.get_loc();
        self.tasks.iter().filter_map(move |(task_name, task)| {
            let task_loc = proj_loc.extend_task(task_name);
            if (skip_disabled && (task.disabled || task.jobs.iter().all(|j| j.disabled)))
                || (!task.parallel && task.doesnt_match(tags))
            {
                None
            } else if task.parallel {
                let iter = task
                    .jobs
                    .iter()
                    .enumerate()
                    .filter_map(move |(job_num, job)| {
                        if (skip_disabled && job.disabled) || job.doesnt_match(tags) {
                            None
                        } else {
                            let job_loc = task_loc.extend_job(job_num);
                            Some((job_loc, job))
                        }
                    });
                Some(TaskOrJob::Job(iter))
            } else {
                Some(TaskOrJob::Task((task_loc, task)))
            }
        })
    }
}

impl Project {
    pub fn new(project: &str) -> io::Result<Self> {
        let path = {
            let mut path = PathBuf::from("/etc/chithi/");
            path.push(format!("{}.toml", project));
            path
        };
        let file = std::fs::read_to_string(&path);
        let file = match file {
            Ok(s) => s,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                error!("{} not found", path.display());
                return Err(e);
            }
            Err(e) => return Err(e),
        };

        toml::from_str(&file).map_err(|e| {
            error!("could not parse project toml {}: {e}", path.display());
            io::Error::other(format!(
                "could not parse project toml {}: {e}",
                path.display()
            ))
        })
    }
    pub fn normalize(self, proj_name: &str) -> io::Result<NormalizedProject> {
        let proj_loc = Loc::new(proj_name);
        Self::check_command_maybe(&self.default_project_command, &proj_loc)?;
        let tasks: io::Result<HashMap<String, NormalizedTask>> = self.tasks.into_iter().map(|(task_name, mut task)| {
            let task_loc = proj_loc.extend_task(&task_name);
            Self::check_command_maybe(&task.default_task_command, &task_loc)?;
            let task_command = task
                .default_task_command
                .or_else(|| self.default_project_command.clone());
            let task_disabled = task.disabled || self.disabled;
            if task.tags.is_empty() && task.jobs.len() == 1 && let Some(job) = task.jobs.first_mut() {
                task.tags.append(&mut job.tags);
            };
            for tag in &task.tags {
                Self::check_tag(tag)?;
            }
            let jobs: Vec<_> = task.jobs.into_iter().enumerate().map(|(job_num, mut job)| {
                let job_loc = task_loc.extend_job(job_num);
                Self::check_command_maybe(&job.command, &job_loc)?;
                for tag in &job.tags {
                    Self::check_tag(tag)?;
                };
                let job_command = job.command.or_else(|| task_command.clone());
                if let Some(command) = job_command {
                    Self::check_sync_job(&command, &job_loc, job.source.is_some() && job.target.is_some())?;
                    if !task.parallel && !job.tags.is_empty() {
                        error!("jobs in sequential tasks with more than 1 job should not have tags, tag set for {job_loc}");
                        return Err(io::Error::other(format!("jobs in sequential tasks with more than 1 job should not have tags, tag set for {job_loc}")));
                    };
                    if task.parallel && task.on_success.is_some() && !job.tags.is_empty() {
                        error!("jobs in parallel tasks with an on-success command should not have tags, tag set for {job_loc}");
                        return Err(io::Error::other(format!("jobs in parallel tasks with an on-success command should not have tags, tag set for {job_loc}")));
                    }
                    job.tags.extend_from_slice(&task.tags);
                    Ok(NormalizedJob {
                        command,
                        disabled: job.disabled || task_disabled,
                        source: job.source,
                        target: job.target,
                        on_success: job.on_success,
                        tags: job.tags.into_iter().collect(),
                    })
                } else {
                    error!("command not set for {job_loc}, please set a command at the job, task, or project level");
                    Err(io::Error::other(format!("command not set for task {job_loc}, please set a command at the job, task, or project level")))
                }
            }).collect::<Result<_,_>>()?;
            Ok((task_name, NormalizedTask {
                            disabled: task_disabled,
                            parallel: task.parallel,
                            on_success: task.on_success,
                            jobs,
                            tags: task.tags.into_iter().collect(),
                        }))
        }).collect();
        tasks.map(|tasks| NormalizedProject {
            name: proj_name.to_string(),
            disabled: self.disabled,
            run_config: self.run.unwrap_or_default(),
            tasks,
        })
    }
    fn check_tag(tag: &str) -> io::Result<()> {
        if tag.is_empty() {
            error!("empty string should not be used as a tag");
            return Err(io::Error::other("found empty string tag in project"));
        }
        if tag.starts_with(['/', '!']) {
            error!("tags should not begin with '/' or '!'");
            return Err(io::Error::other(
                "found tag in project that starts with '/' or '!'",
            ));
        }
        const RESERVED: [&str; 10] = [
            "none", "any", "all", "and", "or", "not", "|", "||", "&", "&&",
        ];
        if RESERVED.contains(&tag) {
            error!(
                "none, any, all, and, or, not, '||', '|', '&', '&&' are reserved and should not be used as tags"
            );
            return Err(io::Error::other(format!(
                "use a reserved word as a tag in project '{}'",
                tag
            )));
        }
        if tag
            .chars()
            .any(|c| c == ',' || c == '(' || c == ')' || c == '"' || c == '\'' || c.is_whitespace())
        {
            error!(
                "tags cannot contain commas, parentheses, quotes, or whitespace characters, invalid tag \"{}\"",
                tag.escape_default()
            );
            return Err(io::Error::other(format!(
                "invalid tag \"{}\"",
                tag.escape_default()
            )));
        }
        Ok(())
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
    pub fn display_label(&self) -> impl std::fmt::Display {
        LocLabel {
            task_name: self.task_name,
            job_num: self.job_num,
        }
    }
    #[cfg(any(feature = "run-bin", feature = "run-bundle"))]
    pub fn create_pidfile(&self) -> io::Result<PidFile> {
        const VAR_RUN: &str = "/var/run/chithi";
        let pidfile_path = if self.task_name.is_some() {
            let proj_dir = std::path::PathBuf::from(VAR_RUN).join(self.proj_name);
            std::fs::create_dir_all(&proj_dir)?;
            proj_dir.join(format!("{}.pid", self.display_label()))
        } else {
            let chithi_dir = std::path::PathBuf::from(VAR_RUN);
            std::fs::create_dir_all(&chithi_dir)?;
            chithi_dir.join(format!("{}.pid", self.proj_name))
        };
        let pidfile = PidFile::new(pidfile_path)?;
        Ok(pidfile)
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

struct LocLabel<'proj> {
    task_name: Option<&'proj str>,
    job_num: Option<usize>,
}

impl<'proj> std::fmt::Display for LocLabel<'proj> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(task_name) = self.task_name {
            write!(f, "{task_name}")?
        };
        if let Some(job_num) = self.job_num {
            write!(f, ".{job_num}")?
        };
        Ok(())
    }
}

#[cfg(any(feature = "run-bin", feature = "run-bundle"))]
/// A PidFile that is closed and unlinked when the object goes out of scope
pub struct PidFile {
    file: std::fs::File,
}

#[cfg(any(feature = "run-bin", feature = "run-bundle"))]
impl PidFile {
    pub fn new(path: std::path::PathBuf) -> io::Result<Self> {
        use std::{io::Write, os::unix::fs::OpenOptionsExt};

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .mode(0o644)
            .open(path.as_path())?;
        file.try_lock()?;
        file.set_len(0)?;
        let pid = format!("{}", std::process::id());
        file.write_all(pid.as_bytes())?;
        Ok(Self { file })
    }
}

#[cfg(any(feature = "run-bin", feature = "run-bundle"))]
impl Drop for PidFile {
    fn drop(&mut self) {
        let _ = self.file.set_len(0);
    }
}
