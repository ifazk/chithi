use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "run")]
#[command(version, about = "Task Runner for chithi", long_about = None)]
pub struct RunArgs {
    /// Runs the without any delays or restarts. Useful when called from a
    /// script that handles its own delays and restarts.
    #[arg(long)]
    pub no_run_config: bool,

    /// Name of project. The runner will look for a .toml file with this name in /etc/chithi/
    #[arg(long, default_value = "chithi")]
    pub project: String,

    pub task: Option<String>,

    pub job: Option<usize>,
}
