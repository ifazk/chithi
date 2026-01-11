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

use crate::compress::Compress;
use crate::send_recv_opts::{OptionsLine, Opts};
use crate::zfs;
use bw::Bytes;
use chrono::format::StrftimeItems;
use clap::{Parser, Subcommand};
use regex_lite::Regex;
use std::collections::HashSet;
use std::ffi::OsString;

mod bw;
pub mod run;

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
    /// Replicates a dataset to another pool
    Sync(Args),
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
pub struct Args {
    /// Compresses data during transfer. Currently accepted options are gzip,
    /// pigz-fast, pigz-slow, zstd-fast, zstdmt-fast, zstd-slow, zstdmt-slow,
    /// lz4, xz, lzo & none
    #[arg(long, default_value = "lzo", value_name = "FORMAT", value_parser = Compress::try_from_str)]
    pub compress: Compress,

    /// Extra identifier which is included in the snapshot name. Can be used for
    /// replicating to multiple targets.
    #[arg(long, value_name = "EXTRA", value_parser = Args::validate_identifier)]
    pub identifier: Option<String>,

    /// Also transfers child datasets
    #[arg(short, long)]
    pub recursive: bool,

    /// Skips syncing of the parent dataset. Does nothing without '--recursive' option.
    #[arg(long, requires = "recursive")]
    pub skip_parent: bool,

    /// Bandwidth limit in bytes/kbytes/etc per second on the source transfer
    #[arg(long, value_parser = Bytes::try_from_str)]
    pub source_bwlimit: Option<Bytes>,

    /// Bandwidth limit in bytes/kbytes/etc per second on the target transfer
    #[arg(long, value_parser = Bytes::try_from_str)]
    pub target_bwlimit: Option<Bytes>,

    /// Specify the mbuffer size, please refer to mbuffer(1) manual page.
    #[arg(long, default_value = "16M", value_name = "VALUE")]
    pub mbuffer_size: String,

    /// Configure how pv displays the progress bar
    #[arg(long, default_value = "-p -t -e -r -b", value_name = "OPTIONS")]
    pub pv_options: String,

    /// Replicates using newest snapshot instead of intermediates
    #[arg(long)]
    pub no_stream: bool,

    /// Timestamp format. All invalid characters in the format will be dropped.
    /// Formatting details can be found in the chrono::format::strftime
    /// documentation.
    #[arg(long, default_value="%Y-%m-%d:%H:%M:%S-GMT%:z", value_parser = Args::validate_timestamp)]
    pub timestamp_format: String,

    /// Does not create new snapshot, only transfers existing
    #[arg(long)]
    pub no_sync_snap: bool,

    /// Does not prune sync snaps at the end of transfers
    #[arg(long)]
    pub keep_sync_snap: bool,

    /// Creates a zfs bookmark for the newest snapshot on source after replication succeeds.
    /// Unless --syncoid-bookmarks is set, the bookmark name includes the
    /// identifier if set.
    #[arg(long)]
    pub create_bookmark: bool,

    /// Use the sanoid/syncoid 2.3 bookmark behaviour. This should be treated as
    /// an experinmental feature, and may not be kept in future minor revisions.
    #[arg(long, requires = "no_sync_snap", requires = "create_bookmark")]
    pub syncoid_bookmarks: bool,

    /// Use "syncoid:sync" property to check if we should sync sync. This should
    /// be treated as an experinmental feature, and may not be kept in future
    /// minor revisions.
    #[arg(long)]
    pub syncoid_sync_check: bool,

    /// If transfer creates new sync snaps, this option chooses what kind of
    /// snapshot formats to prune at the end of transfers. Current options are
    /// syncoid and chithi. Needs to be passed multiple times for multiple
    /// formats.
    #[arg(
        long = "prune-format",
        default_value = "chithi",
        value_name = "SNAPFORMAT"
    )]
    pub prune_formats: Vec<String>,

    /// Adds a hold to the newest snapshot on the source and target after
    /// replication and removes the hold after the next successful replication.
    /// The hold name includes the identifier if set. This allows for separate
    /// holds in case of multiple targets. Can be optionally passed the value
    /// "syncoid" to make syncoid compatible holds.
    #[arg(long, default_value = "false", default_missing_value = "true", value_parser = ["true", "false", "syncoid"], num_args = 0..=1)]
    pub use_hold: String,

    /// Preserves the recordsize on inital sends to the target
    #[arg(long, conflicts_with = "preserve_properties")]
    pub preserve_recordsize: bool,

    /// Preserves locally set dataset properties similar to the zfs send -p
    /// flag, but will also work for encrypted datasets in non raw sends.
    /// Properties are manually fetched on the source and manually written to on
    /// the target, with a blacklist of properties that cannot be written.
    #[arg(long, conflicts_with = "preserve_recordsize")]
    pub preserve_properties: bool,

    /// Does not rollback snapshots on target (it probably requires a readonly target)
    #[arg(long)]
    pub no_rollback: bool,

    /// With this argument, snapshots which are missing on the source will be
    /// destroyed on the target. Use this if you only want to handle snapshots
    /// on the source.
    #[arg(long)]
    pub delete_target_snapshots: bool,

    /// Exclude specific datasets that match the given regular expression. Can be specified multiple times.
    #[arg(long, value_name = "REGEX")]
    pub exclude_datasets: Vec<Regex>,

    /// Exclude specific snapshots that match the given regular expression. Can be specified multiple times. If a snapshot matches both exclude-snaps and include-snaps patterns, then it will be excluded.
    #[arg(long, value_name = "REGEX")]
    pub exclude_snaps: Vec<Regex>,

    /// Only include snapshots that match the given regular expression. Can be specified multiple times. If a snapshot matches both exclude-snaps and include-snaps patterns, then it will be excluded.
    #[arg(long, value_name = "REGEX")]
    pub include_snaps: Vec<Regex>,

    /// Use bookmarks for incremental syncing. When set to "always" (assumed if
    /// no value is passed), we fetch bookmarks as well as snapshots when
    /// computing incremental sends.
    #[arg(long, default_value = "fallback", default_missing_value = "always", value_parser = ["always", "fallback"], num_args = 0..=1)]
    pub use_bookmarks: String,

    /// Prune bookmarks. Bookmarks are not
    #[arg(long, conflicts_with = "syncoid_bookmarks")]
    pub max_bookmarks: Option<std::num::NonZero<usize>>,

    /// Use advanced options for zfs send (the arguments are filtered as needed), e.g. chithi --send-options="Lc e" sets zfs send -L -c -e ...
    #[arg(long, value_name = "OPTIONS", value_parser = Opts::try_from_str, default_value_t)]
    pub send_options: Opts<Vec<OptionsLine<String>>>,

    /// Use advanced options for zfs receive (the arguments are filtered as needed), e.g. chithi --recv-options="ux recordsize o compression=lz4" sets zfs receive -u -x recordsize -o compression=lz4 ...
    #[arg(long, value_name = "OPTIONS", value_parser = Opts::try_from_str, default_value_t)]
    pub recv_options: Opts<Vec<OptionsLine<String>>>,

    /// Passes CIPHER to ssh to use a particular cipher set.
    #[arg(short = 'c', long, value_name = "CIPHER")]
    pub ssh_cipher: Option<String>,

    /// Connects to remote machines on a particular port.
    #[arg(short = 'P', long, value_name = "PORT")]
    pub ssh_port: Option<String>,

    /// Uses config FILE for connecting to remote machines over ssh.
    #[arg(short = 'F', long, value_name = "FILE")]
    pub ssh_config: Option<String>,

    /// Uses identity FILE to connect to remote machines over ssh.
    #[arg(short = 'i', long, value_name = "FILE")]
    pub ssh_identity: Option<String>,

    /// Passes OPTION to ssh for remote usage. Can be specified multiple times
    #[arg(short = 'o', long = "ssh-option", value_name = "OPTION")]
    pub ssh_options: Vec<String>,

    /// Prints out a lot of additional information during a chithi run. Logs overridden by --quiet and RUST_LOG environment variable
    #[arg(long)]
    pub debug: bool,

    /// Supresses non-error output and progress bars. Logs overridden by RUST_LOG environment variable
    #[arg(long)]
    pub quiet: bool,

    /// Dumps a list of snapshots during the run
    #[arg(long)]
    pub dump_snaps: bool,

    /// Passes OPTION to ssh for remote usage. Can be specified multiple times
    #[arg(long)]
    pub no_command_checks: bool,

    /// A comma separated list of optional commands to skip. Current values are:
    /// sourcepv localpv targetpv compress localcompress sourcembuffer
    /// targetmbuffer localmbuffer
    #[arg(long, value_parser = Args::get_commands_to_skip, default_value = "")]
    pub skip_optional_commands: HashSet<&'static str>,

    /// Do a dry run, without modifying datasets and pools. The dry run
    /// functionality is provided on a best effort basis and may break between
    /// minor versions.
    #[arg(long)]
    pub dry_run: bool,

    /// Don't use the ZFS resume feature if available
    #[arg(long)]
    pub no_resume: bool,

    /// Don't try to recreate clones on target. Clone handling is done by
    /// deferring child datasets that are clones to a second pass of syncing, so
    /// this flag is not meaningful without the --recursive flag.
    #[arg(long, requires = "recursive")]
    pub no_clone_handling: bool,

    /// Bypass the root check, for use with ZFS permission delegation
    #[arg(long)]
    pub no_privilege_elevation: bool,

    /// Manually specifying source host (and user)
    #[arg(long)]
    pub source_host: Option<String>,

    /// Manually specifying target host (and user)
    #[arg(long)]
    pub target_host: Option<String>,

    /// Remove target datasets recursively if there are no matching snapshots/bookmarks (also overwrites conflicting named snapshots)
    #[arg(long)]
    pub force_delete: bool,

    /// Prevents the recursive recv check at the start of the sync
    #[arg(long, requires = "recursive")]
    pub no_recv_check_start: bool,

    pub source: String,

    pub target: String,
}

impl Args {
    pub fn clone_handling(&self) -> bool {
        !self.no_clone_handling
    }
    pub fn recv_check_start(&self) -> bool {
        !self.no_recv_check_start
    }
    /// Fills in the optional_commands_to_skip field
    fn get_commands_to_skip(commands: &str) -> Result<HashSet<&'static str>, String> {
        let mut res = HashSet::new();
        let commands = commands.trim();
        if commands.is_empty() {
            return Ok(res);
        }
        for command in commands.split(',') {
            match command {
                "sourcepv" => {
                    res.insert("sourcepv");
                }
                "targetpv" => {
                    res.insert("targetpv");
                }
                "localpv" => {
                    res.insert("localpv");
                }
                "compress" => {
                    res.insert("compress");
                }
                "localcompress" => {
                    res.insert("localcompress");
                }
                "sourcembuffer" => {
                    res.insert("sourcembuffer");
                }
                "targetmbuffer" => {
                    res.insert("targetmbuffer");
                }
                "localmbuffer" => {
                    res.insert("localmbuffer");
                }
                s => {
                    return Err(format!(
                        "unsupported command {s} passed to --skip-optional-commands"
                    ));
                }
            }
        }
        Ok(res)
    }

    pub fn optional_enabled(&self, optional: &'static str) -> bool {
        !self.skip_optional_commands.contains(optional)
    }

    pub fn get_pv_options(&self) -> Vec<OsString> {
        self.pv_options
            .split_whitespace()
            .map(Into::into)
            .collect::<Vec<_>>()
    }

    pub fn get_source_mbuffer_args(&self) -> Vec<OsString> {
        let mut args = vec!["-q", "-s", "128k", "-m", self.mbuffer_size.as_str()];
        if let Some(limit) = &self.source_bwlimit {
            args.push("-R");
            args.push(&limit.str);
        }
        args.iter().map(Into::into).collect()
    }

    pub fn get_target_mbuffer_args(&self) -> Vec<OsString> {
        let mut args = vec!["-q", "-s", "128k", "-m", self.mbuffer_size.as_str()];
        if let Some(limit) = &self.target_bwlimit {
            args.push("-r");
            args.push(&limit.str);
        }
        args.iter().map(Into::into).collect()
    }

    pub fn get_timestamp(&self) -> String {
        let now = chrono::Local::now();
        let formatted = format!("{}", now.format(&self.timestamp_format));
        formatted
            .chars()
            .filter(|&c| zfs::is_component_char(c))
            .collect()
    }

    /// Returns false for now. In the future, we might allow direct ssh/tls (or
    /// even insecure tcp) connections between remote hosts.
    pub fn direct_connection(&self) -> bool {
        false
    }

    fn validate_identifier(value: &str) -> Result<String, &'static str> {
        fn invalid_char(c: char) -> bool {
            !zfs::is_component_char(c)
        }
        if value.contains(invalid_char) {
            Err("extra indentifier contains invalid chars!")
        } else {
            Ok(value.to_string())
        }
    }

    fn validate_timestamp(value: &str) -> Result<String, &'static str> {
        if StrftimeItems::new(value).parse().is_ok() {
            return Ok(value.to_string());
        }
        Err("invalid timestamp format see chrono time formats")
    }
}
