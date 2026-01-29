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

use clap::{Parser, Subcommand};
use std::ffi::OsString;

#[cfg(feature = "list")]
pub mod list;
#[cfg(any(feature = "run-bin", feature = "run-bundle"))]
pub mod run;
pub mod sync;
#[cfg(any(feature = "run-bin", feature = "run-bundle", feature = "list"))]
pub mod tags;

#[derive(Debug, Parser)]
#[command(name = "chithi")]
#[command(version, about = "ZFS snapshot replication tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Replicates a dataset to another pool.
    Sync(sync::SyncArgs),
    #[cfg(feature = "list")]
    /// Lists tasks and jobs in a chithi project.
    List(list::ListArgs),
    #[cfg(feature = "run-bundle")]
    /// Task runner.
    Run(run::RunArgs),
    #[command(external_subcommand)]
    External(Vec<OsString>),
}
