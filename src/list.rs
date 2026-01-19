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

use crate::args::list::ListArgs;
use crate::spec::{Loc, NormalizedJob, NormalizedTask, Project, TaskOrJob, TaskOrJobIter};
use crate::util::{OptDisplay, SpaceSeparatedStrings};
use log::{error, info};
use std::io::{self, Write};
use tabwriter::TabWriter;

pub fn main(args: ListArgs) -> io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .format_target(false)
        .init();

    let proj = Project::new(&args.project)?;
    let proj = proj.normalize(&args.project)?;

    if args.skip_disabled && proj.disabled {
        info!("Project is disabled and you asked to skip all disabled tasks and jobs");
        return Ok(());
    }

    if let Some(task_name) = args.task {
        if let Some(task) = proj.tasks.get(&task_name) {
            if args.skip_disabled && task.disabled {
                info!(
                    "Task {task_name} is disabled and you asked to skip all disabled tasks and jobs"
                );
                return Ok(());
            }
            let task_loc = proj.get_loc().extend_task(&task_name);
            let jobs = task.jobs.iter().enumerate().filter_map(|(job_num, job)| {
                if args.skip_disabled && job.disabled {
                    None
                } else {
                    Some((task_loc.extend_job(job_num), job))
                }
            });
            let jobs = TaskOrJob::Job(jobs);
            let iter = std::iter::once(jobs);
            print_listings(args.long, !args.no_headers, iter)?
        } else {
            error!("Task {task_name} not found in project {}", proj.name);
            return Err(io::Error::other(format!(
                "Task {task_name} not found in project {}",
                proj.name
            )));
        }
    } else {
        let independents = proj.list_independents(args.skip_disabled);
        print_listings(args.long, !args.no_headers, independents)?
    }

    Ok(())
}

fn print_listings<
    'proj,
    I: Iterator<
        Item = TaskOrJob<
            impl Iterator<Item = (Loc<'proj, 'proj>, &'proj NormalizedJob)>,
            (Loc<'proj, 'proj>, &'proj NormalizedTask),
        >,
    >,
>(
    long_listing: bool,
    headers: bool,
    iter: I,
) -> io::Result<()> {
    if long_listing {
        let loc_disabled_job = iter.map(|j| match j {
            TaskOrJob::Job(iter) => {
                TaskOrJob::Job(iter.map(|(job_loc, job)| (job_loc, job.disabled, Some(job))))
            }
            TaskOrJob::Task((task_loc, task)) => {
                let mut disabled = task.disabled;
                let job = if task.jobs.len() == 1 {
                    disabled = task.jobs[0].disabled;
                    task.jobs.first()
                } else {
                    None
                };
                TaskOrJob::Task((task_loc, disabled, job))
            }
        });
        let iter = TaskOrJobIter::new(loc_disabled_job);
        if headers {
            let mut tw = TabWriter::new(io::stdout());
            print_long_listing(iter, headers, &mut tw)?;
            tw.flush()?;
        } else {
            let mut stdout = io::stdout();
            print_long_listing(iter, headers, &mut stdout)?;
            stdout.flush()?;
        }
    } else {
        for i in iter {
            match i {
                TaskOrJob::Task((task_loc, _)) => {
                    println!("{}", task_loc.display_label())
                }
                TaskOrJob::Job(iter) => {
                    for (job_loc, _) in iter {
                        println!("{}", job_loc.display_label())
                    }
                }
            }
        }
    }
    Ok(())
}

fn print_long_listing<
    'proj,
    T: Iterator<Item = (Loc<'proj, 'proj>, bool, Option<&'proj NormalizedJob>)>,
    W: io::Write,
>(
    iter: T,
    headers: bool,
    w: &mut W,
) -> io::Result<()> {
    if headers {
        writeln!(w, "Label\tdisabled\tSource\tTarget\tCommand")?
    }
    for (loc, disabled, job_maybe) in iter {
        let source_maybe = OptDisplay(job_maybe.and_then(|j| j.source.as_ref()));
        let target_maybe = OptDisplay(job_maybe.and_then(|j| j.target.as_ref()));
        let command = job_maybe.map(|j| {
            SpaceSeparatedStrings(
                j.command
                    .iter()
                    .map(|x| format!("\"{}\"", x.escape_default()))
                    .collect::<Vec<_>>(),
            )
        });
        let command_maybe = OptDisplay(command.as_ref());
        writeln!(
            w,
            "{}\t{disabled}\t{source_maybe}\t{target_maybe}\t{command_maybe}",
            loc.display_label()
        )?
    }
    Ok(())
}
