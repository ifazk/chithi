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

use crate::spec::{Loc, NormalizedJob, Project, Seconds};
use crate::{args::run::RunArgs, spec::RunConfig};
use log::{error, info};
use std::collections::HashMap;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;
use std::{io, path::PathBuf};

pub fn main(args: RunArgs) -> io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .format_target(false)
        .init();

    let path = {
        let mut path = PathBuf::from("/etc/chithi/");
        path.push(format!("{}.toml", args.project));
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

    let proj: Project = toml::from_str(&file).map_err(|e| {
        error!("could not parse project toml {}: {e}", path.display());
        io::Error::other(format!(
            "could not parse project toml {}: {e}",
            path.display()
        ))
    })?;
    let proj = proj.normalize(&args.project)?;

    let (task_maybe, job_maybe) = match args.task_or_job.as_deref() {
        Some(task_or_job) => {
            if let Some(n) = task_or_job.find('.') {
                let task_str = &task_or_job[..n];
                let job_str = &task_or_job[n + 1..];
                let job = job_str.parse::<usize>().map_err(|e| {
                    error!("could not parse job number from {job_str}: {e}");
                    io::Error::other(format!("could not parse job number from {job_str}: {e}"))
                })?;
                (Some(task_str), Some(job))
            } else {
                (Some(task_or_job), None)
            }
        }
        None => (None, None),
    };

    // The runner does one of a fews things:
    // 1. Run every task in a project if there's no task provided
    // 2. Run a single sequential task with or without run config
    // 3. Run a parallel task with run config
    // 4. Run a single job with or without run config
    //
    // The parallel tasks must be self calls, since the only reasonable
    // non-blocking posix api available to us is waitid. When running parallel
    // tasks using an external runner like systemd which handles it's own delays
    // and restarts, individual jobs must be run with --no-run-config in
    // parallel.
    //
    // When scheduling jobs using systemd or other scheduling systems that have
    // their own restarts, etc, you have to schedule either sequential tasks or
    // individual jobs using --no-run-config.

    if proj.disabled {
        info!("not running disabled project {}", args.project);
        return Ok(());
    }

    let proj_loc = proj.get_loc();

    let parallel_jobs = match (task_maybe, job_maybe) {
        (None, _) => proj.get_enabled_tasks_or_jobs().collect::<Vec<_>>(),
        (Some(task_name), None) => {
            // Task check
            let task_loc = proj_loc.extend_task(task_name);
            let Some(task) = proj.tasks.get(task_name) else {
                error!("could not find {task_loc}");
                return Err(io::Error::other(format!("could not find {task_loc}")));
            };
            if task.disabled {
                info!("not running disabled {task_loc}");
                return Ok(());
            }
            let all_disabled = task.jobs.iter().all(|j| j.disabled);
            if task.jobs.len() != 1 && all_disabled {
                // extra info when task jobs != 1 and there are no enabled jobs
                info!("no enabled jobs for {task_loc}");
            }
            if task.parallel {
                // parallel (with config)
                task.get_enabled_jobs(task_loc).collect()
            } else if args.no_run_config {
                // sequential without config
                if all_disabled {
                    return Ok(());
                }
                for (job_num, job) in task.jobs.iter().enumerate() {
                    if job.disabled {
                        continue;
                    }
                    let status = job.run()?;
                    if !status.success() {
                        let job_loc = task_loc.extend_job(job_num);
                        error!("{job_loc} exited with {}", status);
                        return Err(io::Error::other(format!(
                            "{job_loc} exited with {}",
                            status
                        )));
                    }
                }
                return Ok(());
            } else {
                // sequential with config
                if all_disabled {
                    return Ok(());
                }
                proj.run_config.inital_delay(task_loc);
                for (job_num, job) in task.jobs.iter().enumerate() {
                    if job.disabled {
                        continue;
                    }
                    let job_loc = task_loc.extend_job(job_num);
                    run_job_with_config(&proj.run_config, job_loc, job)?
                }
                return Ok(());
            }
        }
        (Some(task_name), Some(job_num)) => {
            // Task check
            let task_loc = proj_loc.extend_task(task_name);
            let Some(task) = proj.tasks.get(task_name) else {
                error!("could not find {task_loc}");
                return Err(io::Error::other(format!("could not find {task_loc}")));
            };
            if task.disabled {
                info!("not running disabled {task_loc}");
                return Ok(());
            }
            // Job check
            let job_loc = task_loc.extend_job(job_num);
            let Some(job) = task.jobs.get(job_num) else {
                error!("no such job: {job_loc}");
                return Err(io::Error::other(format!("no such job: {job_loc}")));
            };
            if job.disabled {
                info!("not running disabled {job_loc}");
                return Ok(());
            };
            // Job run
            if args.no_run_config {
                let err = job.get_command().exec();
                error!("running {job_loc} failed with {err}");
                return Err(err);
            } else {
                proj.run_config.inital_delay(job_loc);
                return run_job_with_config(&proj.run_config, job_loc, job);
            }
        }
    };

    let mut job_handles = HashMap::new();

    let (program, add_run): (std::ffi::OsString, _) = match std::env::current_exe() {
        Ok(path)
            if path.file_name().is_some()
                && (path.file_name().unwrap() == "chithi"
                    || path.file_name().unwrap() == "chithi-run") =>
        {
            let add_run = path.file_name().unwrap() == "chithi";
            (path.into(), add_run)
        }
        Ok(_) | Err(_) => {
            // default to running chithi
            ("chithi".into(), true)
        }
    };

    for j in parallel_jobs {
        let mut cmd = Command::new(&program);
        if add_run {
            cmd.arg("run");
        };
        cmd.arg("--project");
        cmd.arg(j.proj_name);
        if let Some(task) = j.task_name {
            if let Some(job_num) = j.job_num {
                cmd.arg(format!("{task}.{job_num}"));
            } else {
                cmd.arg(task);
            }
        } else {
            error!("internal error: did not find task name in job list");
        }
        match cmd.spawn() {
            Ok(handle) => {
                let id = handle.id();
                job_handles.insert(id, handle);
            }
            Err(e) => {
                error!("could not recursively start {j}: {e}");
            }
        }
    }

    while !job_handles.is_empty() {
        let id = waitid_all()? as u32;
        match job_handles.remove(&id) {
            Some(mut handle) => {
                handle.wait()?;
            }
            None => {
                error!("unregistered child {id} exited, giving up");
                return Err(io::Error::other("unregisted child exited"));
            }
        };
    }

    Ok(())
}

pub fn run_job_with_config(
    run_config: &RunConfig,
    job_loc: Loc,
    job: &NormalizedJob,
) -> io::Result<()> {
    for i in 0usize.. {
        let mut command = job.get_command();
        command.stdin(Stdio::null());
        match command.status() {
            Ok(e) if e.success() => {
                return Ok(());
            }
            Ok(e) => {
                error!("{job_loc} exited with {e}");
                if i < run_config.max_restart_count.unwrap_or_default() as usize {
                    error!("restart count for {job_loc} it at {i}");
                    if let Some(delay) = run_config.restart_delay(i) {
                        let secs = rand::random_range(0..delay);
                        if secs > 0 {
                            error!("delaying restart by {}", Seconds(secs));
                            sleep(Duration::from_secs(secs as u64));
                        }
                    }
                    continue;
                } else if run_config.max_restart_count.is_some() {
                    return Err(io::Error::other(format!(
                        "{job_loc} max restarts reached exited with {e}"
                    )));
                } else {
                    return Err(io::Error::other(format!("{job_loc} exited with {e}")));
                }
            }
            Err(e) => {
                error!("running {job_loc} failed with {e}, giving up");
                return Err(io::Error::other(format!(
                    "running {job_loc} failed with {e}, giving up"
                )));
            }
        }
    }
    Ok(())
}

fn waitid_all() -> io::Result<libc::pid_t> {
    let mut infop: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let res = unsafe {
        libc::waitid(
            libc::P_ALL,
            0,
            &mut infop as *mut _,
            libc::WEXITED | libc::WNOWAIT,
        )
    };
    if res == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { infop.si_pid() })
    }
}
