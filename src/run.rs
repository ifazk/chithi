use crate::spec::{NormalizedJob, Project};
use crate::{args::run::RunArgs, spec::RunConfig};
use log::error;
use std::process::Stdio;
use std::thread::sleep;
use std::time::Duration;
use std::{io, path::PathBuf};

pub fn main(args: RunArgs) -> io::Result<()> {
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

    if let Some(inital_delay) = proj.run_config.max_inital_delay_secs {
        println!("max initial delay is {inital_delay}");
    }

    Ok(())
}

/// Simple human readable time
struct Seconds(u16);

impl std::fmt::Display for Seconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut seconds = self.0;
        let hours = seconds / 3600;
        seconds = hours % 3600;
        let minutes = seconds / 60;
        seconds = seconds % 60;
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
        Ok(())
    }
}

pub fn run_job_with_config(run_config: &RunConfig, job: &NormalizedJob) {
    if job.disabled {
        return;
    }

    for i in 0usize.. {
        let mut command = job.get_command();
        command.stdin(Stdio::null());
        match command.status() {
            Ok(e) if e.success() => {
                return;
            }
            Ok(e) => {
                error!("job exited with {e}");
                if i < run_config.max_restart_count.unwrap_or_default() as usize {
                    error!("restart count it at {i}");
                    if let Some(delay) = run_config.restart_delay(i) {
                        let secs = rand::random_range(0..delay);
                        if secs > 0 {
                            error!("delaying restart by {}", Seconds(secs));
                            sleep(Duration::from_secs(secs as u64));
                        }
                    }
                    continue;
                } else {
                    break;
                }
            }
            Err(e) => {
                error!("running job failed with {e}, giving up");
                return;
            }
        }
    }
}
