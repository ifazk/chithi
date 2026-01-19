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

use chithi::args::{Cli, Commands};
#[cfg(feature = "list")]
use chithi::list;
#[cfg(feature = "run-bundle")]
use chithi::run;
use chithi::sync;
use clap::Parser;
use log::error;
use std::{ffi::OsString, io, os::unix::process::CommandExt, process::Command};

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Sync(args) => sync::main(args),
        #[cfg(feature = "list")]
        Commands::List(args) => list::main(args),
        #[cfg(feature = "run-bundle")]
        Commands::Run(args) => run::main(args),
        Commands::External(args) => {
            let mut program = OsString::from("chithi-");
            program.push(&args[0]);
            let mut command = Command::new(&program);
            command.args(&args[1..]);
            let err = command.exec();
            if err.kind() == io::ErrorKind::NotFound {
                env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
                    .format_timestamp(None)
                    .format_target(false)
                    .init();
                error!("{} was not found in PATH", program.display());
            }
            Err(err)
        }
    }
}
