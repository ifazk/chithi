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

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "run")]
#[command(version, about = "Task listings for chithi", long_about = None)]
pub struct ListArgs {
    /// Use a long listing format. Shows disabled, sources, targets, commands.
    #[arg(short, long)]
    pub long: bool,

    /// Scripted mode for long listing. Has no effect if long listing is not enabled.
    #[arg(short = 'H', long)]
    pub no_headers: bool,

    /// Skip disabled tasks and jobs in listings.
    #[arg(long)]
    pub skip_disabled: bool,

    /// Name of project. Chithi will look for a .toml file with this name in /etc/chithi/.
    #[arg(long, default_value = "chithi")]
    pub project: String,

    /// Name of sync task in project (NAME). If no tasks are provided, the
    /// sequential tasks and jobs in parallel tasks will be listed.
    pub task: Option<String>,
}
