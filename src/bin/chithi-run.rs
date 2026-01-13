use chithi::args::run::RunArgs;
use chithi::run;
use clap::Parser;
use std::io;

fn main() -> io::Result<()> {
    let args = RunArgs::parse();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .format_target(false)
        .init();

    run::main(args)
}
