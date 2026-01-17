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

use crate::AutoTerminate;
use crate::args::sync::SyncArgs;
use crate::{Cmd, CmdTarget, Pipeline};
use log::{debug, warn};
use std::{
    collections::{HashMap, HashSet},
    io::{self, IsTerminal},
    process::Stdio,
};

// Because of --skip-optional-commands, we need a bit more infrastructure than
// syncoid for managing the combinatorial explosion in options and checks.

/// This type is used to figure out what optional commands to check for, and
/// then translate those for use in build_sync_pipelines.
#[derive(Clone, Copy)]
enum ConnectionType {
    Local,
    Push,
    Pull,
    RemoteDirect,
    RemoteIndirect,
}

impl ConnectionType {
    fn new<'args>(
        source_cmd_target: &'args CmdTarget<'args>,
        target_cmd_target: &'args CmdTarget<'args>,
        args: &'args SyncArgs,
    ) -> Self {
        match (source_cmd_target.is_remote(), target_cmd_target.is_remote()) {
            (true, true) => {
                if args.direct_connection() {
                    ConnectionType::RemoteDirect
                } else {
                    ConnectionType::RemoteIndirect
                }
            }
            (true, false) => ConnectionType::Pull,
            (false, true) => ConnectionType::Push,
            (false, false) => ConnectionType::Local,
        }
    }

    /// Make all decisions without doing existence checks
    fn get_relevant_enabled(&self, args: &SyncArgs) -> HashSet<&'static str> {
        let mut res = HashSet::new();
        let use_pv = std::io::stderr().is_terminal() && !args.quiet;
        let use_compress = args.compress.is_some();
        match self {
            ConnectionType::Local => {
                //"localpv", "localmbuffer"
                if args.optional_enabled("localpv") && use_pv {
                    res.insert("localpv");
                }
                if args.optional_enabled("localmbuffer") {
                    res.insert("localmbuffer");
                }
            }
            ConnectionType::Push => {
                // "localpv", "localcompress", "localmbuffer", "targetcompress", "targetmbuffer"
                if args.optional_enabled("localpv") && use_pv {
                    res.insert("localpv");
                }
                if args.optional_enabled("localmbuffer") {
                    res.insert("localmbuffer");
                }
                if args.optional_enabled("localcompress")
                    && args.optional_enabled("compress")
                    && use_compress
                {
                    res.insert("localcompress");
                    res.insert("targetcompress");
                }
                if args.optional_enabled("targetmbuffer") {
                    res.insert("targetmbuffer");
                }
            }
            ConnectionType::Pull => {
                // "sourcecompress", "sourcembuffer", "localcompress", "localmbuffer", "localpv"
                if args.optional_enabled("compress")
                    && args.optional_enabled("localcompress")
                    && use_compress
                {
                    res.insert("sourcecompress");
                    res.insert("localcompress");
                }
                if args.optional_enabled("sourcembuffer") {
                    res.insert("sourcembuffer");
                }
                if args.optional_enabled("localmbuffer") {
                    res.insert("localmbuffer");
                }
                if args.optional_enabled("localpv") && !args.quiet {
                    res.insert("localpv");
                }
            }
            ConnectionType::RemoteDirect => {
                // "sourcepv", "sourcecompress", "sourcembuffer", "targetcompress", "targetmbuffer", "targetpv"
                if args.optional_enabled("sourcepv") && use_pv {
                    res.insert("sourcepv");
                }
                if args.optional_enabled("compress") && use_compress {
                    res.insert("compress");
                }
                if args.optional_enabled("sourcembuffer") {
                    res.insert("sourcembuffer");
                }
                if args.optional_enabled("targetmbuffer") {
                    res.insert("targetmbuffer");
                }
                if args.optional_enabled("targetpv") && use_pv {
                    res.insert("targetpv");
                }
            }
            ConnectionType::RemoteIndirect => {
                // "sourcepv", "sourcecompress", "targetmbuffer", "localmbuffer", "localcompress", "localpv", //"localcompress", //"localmbuffer", "targetcompress"
                if args.optional_enabled("sourcepv") && use_pv {
                    res.insert("sourcepv");
                }
                if args.optional_enabled("compress") && use_compress {
                    res.insert("compress");
                    if args.optional_enabled("localcompress")
                        && args.optional_enabled("localpv")
                        && use_pv
                    {
                        res.insert("localcompress");
                        res.insert("localpv");
                    }
                } else if args.optional_enabled("localpv") && use_pv {
                    res.insert("localpv");
                }
                if args.optional_enabled("sourcembuffer") {
                    res.insert("sourcembuffer");
                }
                if args.optional_enabled("targetmbuffer") {
                    res.insert("targetmbuffer");
                }
                if args.optional_enabled("localmbuffer") {
                    res.insert("localmbuffer");
                }
            }
        };
        res
    }
}

struct OptionalCommand<'args> {
    cmd: Cmd<'args>,
    insert_as: &'static str,
    skip_with: &'static str,
}

pub struct OptionalCommands<'args> {
    conn_type: ConnectionType,
    source_cmd_target: &'args CmdTarget<'args>,
    target_cmd_target: &'args CmdTarget<'args>,
    local_cmd_target: &'args CmdTarget<'args>,
    inner: HashMap<&'static str, Cmd<'args>>,
}

type Pipelines<'args, 'cmd> = (
    Pipeline<'args>,
    Option<Pipeline<'args>>,
    Option<Pipeline<'args>>,
);

impl<'args> OptionalCommands<'args> {
    fn check(args: &SyncArgs, cmd: &Cmd) -> io::Result<bool> {
        if args.no_command_checks {
            Ok(true)
        } else {
            Ok(cmd.to_check().output()?.status.success())
        }
    }
    fn not_avail_warn(cmds: &str, targets: &str, continue_without: &str, skip_with: &str) {
        warn!(
            "{} not available on {} - sync will continue without {} - to disable this warning use --skip-optional-commands '{}'",
            cmds, targets, continue_without, skip_with,
        );
    }
    fn sync_warn(cmd: &Cmd, continue_without: &str, skip_with: &str) {
        Self::not_avail_warn(
            cmd.base(),
            cmd.target().pretty_str(),
            continue_without,
            skip_with,
        );
    }
    fn insert_all_checked<const N: usize>(
        &mut self,
        args: &SyncArgs,
        continue_without: &str,
        cmds: [OptionalCommand<'args>; N],
    ) -> io::Result<bool> {
        let mut already_checked = HashSet::new();
        for cmd in &cmds {
            if already_checked.contains(&(cmd.cmd.target().host(), cmd.cmd.base())) {
                continue;
            }
            if Self::check(args, &cmd.cmd)? {
                already_checked.insert((cmd.cmd.target().host(), cmd.cmd.base()))
            } else {
                Self::sync_warn(&cmd.cmd, continue_without, cmd.skip_with);
                return Ok(false);
            };
        }
        for cmd in cmds {
            self.inner.insert(cmd.insert_as, cmd.cmd.to_local());
        }
        Ok(true)
    }
    fn insert_all_delay_warn<const N: usize>(
        &mut self,
        args: &SyncArgs,
        cmds: [OptionalCommand<'args>; N],
        warn: &mut Vec<(&'static str, &'args str, &'static str)>,
    ) -> io::Result<bool> {
        let mut already_checked = HashSet::new();
        for cmd in &cmds {
            if already_checked.contains(&(cmd.cmd.target().host(), cmd.cmd.base())) {
                continue;
            }
            if Self::check(args, &cmd.cmd)? {
                already_checked.insert((cmd.cmd.target().host(), cmd.cmd.base()))
            } else {
                warn.push((cmd.cmd.base(), cmd.cmd.target().pretty_str(), cmd.skip_with));
                return Ok(false);
            };
        }
        for cmd in cmds {
            self.inner.insert(cmd.insert_as, cmd.cmd.to_local());
        }
        Ok(true)
    }
    fn insert_first_no_warn(
        &mut self,
        args: &SyncArgs,
        cmds: [OptionalCommand<'args>; 2],
    ) -> io::Result<bool> {
        for cmd in cmds {
            if Self::check(args, &cmd.cmd)? {
                self.inner.insert(cmd.insert_as, cmd.cmd.to_local());
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn new(
        source_cmd_target: &'args CmdTarget<'args>,
        target_cmd_target: &'args CmdTarget<'args>,
        local_cmd_target: &'args CmdTarget<'args>,
        args: &'args SyncArgs,
    ) -> io::Result<Self> {
        let conn_type = ConnectionType::new(source_cmd_target, target_cmd_target, args);
        let mut res = Self {
            conn_type,
            source_cmd_target,
            target_cmd_target,
            local_cmd_target,
            inner: HashMap::new(),
        };
        let enabled = conn_type.get_relevant_enabled(args);
        // There's a bunch of allocated objects here, and not all of them are
        // used in every case. But it's not all that much in the grand scheme of
        // things.
        let local_pv = Cmd::new_from_vec(local_cmd_target, false, "pv", args.get_pv_options());
        let source_pv = Cmd::new_from_vec(source_cmd_target, false, "pv", args.get_pv_options());
        let target_pv = Cmd::new_from_vec(target_cmd_target, false, "pv", args.get_pv_options());
        let local_source_mbuffer = Cmd::new_from_vec(
            local_cmd_target,
            false,
            "mbuffer",
            args.get_source_mbuffer_args(),
        );
        let local_target_mbuffer = Cmd::new_from_vec(
            local_cmd_target,
            false,
            "mbuffer",
            args.get_target_mbuffer_args(),
        );
        let source_mbuffer = Cmd::new_from_vec(
            source_cmd_target,
            false,
            "mbuffer",
            args.get_source_mbuffer_args(),
        );
        let target_mbuffer = Cmd::new_from_vec(
            target_cmd_target,
            false,
            "mbuffer",
            args.get_target_mbuffer_args(),
        );
        // Okay to call unwrap on these if enabled. Enabled already does a is_some check on args.compress.
        let (local_compress, local_decompress, source_compress, target_decompress) = args
            .compress
            .to_cmd()
            .map(|compress| {
                let local_compress = Cmd::new_from_vec(
                    local_cmd_target,
                    false,
                    compress.base,
                    compress.get_compress_args(),
                );
                let local_decompress = Cmd::new_from_vec(
                    local_cmd_target,
                    false,
                    compress.decompress,
                    compress.get_decompress_args(),
                );
                let source_compress = Cmd::new_from_vec(
                    source_cmd_target,
                    false,
                    compress.base,
                    compress.get_compress_args(),
                );
                let target_decompress = Cmd::new_from_vec(
                    target_cmd_target,
                    false,
                    compress.decompress,
                    compress.get_decompress_args(),
                );
                (
                    Some(local_compress),
                    Some(local_decompress),
                    Some(source_compress),
                    Some(target_decompress),
                )
            })
            .unwrap_or_default();

        // In incoming tags are translated for use in the source/target
        // pipelines. E.g. for Local transfers localpv in translated to
        // sourcepv.
        match conn_type {
            ConnectionType::Local => {
                //"localpv", "localmbuffer"
                if enabled.contains("localpv") {
                    // for local pipeline, we want to use local_mbuffer as the "sourcepv"
                    res.insert_all_checked(
                        args,
                        "progress bar",
                        [OptionalCommand {
                            cmd: local_pv,
                            insert_as: "sourcepv",
                            skip_with: "localpv",
                        }],
                    )?;
                }
                if enabled.contains("localmbuffer") {
                    // for local pipeline, we want to use local_source_mbuffer as the "sourcembuffer"
                    res.insert_all_checked(
                        args,
                        "buffering and bandwith limits",
                        [OptionalCommand {
                            cmd: local_source_mbuffer,
                            insert_as: "sourcembuffer",
                            skip_with: "localmbuffer",
                        }],
                    )?;
                }
            }
            ConnectionType::Push => {
                // "localpv", "localcompress", "localmbuffer", "targetcompress", "targetmbuffer"
                if enabled.contains("localpv") {
                    // for push pipeline, we want to use local_pv as the "sourcepv"
                    res.insert_all_checked(
                        args,
                        "progress bar",
                        [OptionalCommand {
                            cmd: local_pv,
                            insert_as: "sourcepv",
                            skip_with: "localpv",
                        }],
                    )?;
                }
                if enabled.contains("localmbuffer") {
                    // for push pipeline, we want to use local_source_mbuffer as the "sourcembuffer"
                    res.insert_all_checked(
                        args,
                        "buffering and bandwith limits",
                        [OptionalCommand {
                            cmd: local_source_mbuffer,
                            insert_as: "sourcembuffer",
                            skip_with: "localmbuffer",
                        }],
                    )?;
                }
                if enabled.contains("localcompress") && enabled.contains("compress") {
                    res.insert_all_checked(
                        args,
                        "compression",
                        [
                            OptionalCommand {
                                cmd: local_compress.unwrap(),
                                insert_as: "sourcecompress",
                                skip_with: "localcompress",
                            },
                            OptionalCommand {
                                cmd: target_decompress.unwrap(),
                                insert_as: "targetcompress",
                                skip_with: "compress",
                            },
                        ],
                    )?;
                }
                if enabled.contains("targetmbuffer") {
                    res.insert_all_checked(
                        args,
                        "buffering and write limits",
                        [OptionalCommand {
                            cmd: target_mbuffer,
                            insert_as: "targetmbuffer",
                            skip_with: "targetmbuffer",
                        }],
                    )?;
                }
            }
            ConnectionType::Pull => {
                // "sourcecompress", "sourcembuffer", "localcompress", "localmbuffer", "localpv"
                if enabled.contains("compress") && enabled.contains("localcompress") {
                    res.insert_all_checked(
                        args,
                        "compression",
                        [
                            OptionalCommand {
                                cmd: source_compress.unwrap(),
                                insert_as: "sourcecompress",
                                skip_with: "compress",
                            },
                            OptionalCommand {
                                cmd: local_decompress.unwrap(),
                                insert_as: "targetcompress",
                                skip_with: "localcompress",
                            },
                        ],
                    )?;
                }
                if enabled.contains("sourcembuffer") {
                    res.insert_all_checked(
                        args,
                        "buffering and bandwith limits",
                        [OptionalCommand {
                            cmd: source_mbuffer,
                            insert_as: "sourcembuffer",
                            skip_with: "sourcembuffer",
                        }],
                    )?;
                }
                if enabled.contains("localmbuffer") {
                    res.insert_all_checked(
                        args,
                        "buffering and write limits",
                        [OptionalCommand {
                            cmd: local_target_mbuffer,
                            insert_as: "targetmbuffer",
                            skip_with: "localmbuffer",
                        }],
                    )?;
                }
                if enabled.contains("localpv") {
                    // for pull pipeline, we want to use local_pv as the "targetpv"
                    res.insert_all_checked(
                        args,
                        "progress bar",
                        [OptionalCommand {
                            cmd: local_pv,
                            insert_as: "targetpv",
                            skip_with: "localpv",
                        }],
                    )?;
                }
            }
            ConnectionType::RemoteDirect => {
                // "sourcepv", "sourcecompress", "sourcembuffer", "targetcompress", "targetmbuffer", "targetpv"
                if enabled.contains("sourcepv") && enabled.contains("targetpv") {
                    let source_pretty = source_pv.target().pretty_str();
                    let target_pretty = target_pv.target().pretty_str();
                    let pvs = [
                        OptionalCommand {
                            cmd: source_pv,
                            insert_as: "sourcepv",
                            skip_with: "sourcepv",
                        },
                        OptionalCommand {
                            cmd: target_pv,
                            insert_as: "targetpv",
                            skip_with: "targetpv",
                        },
                    ];
                    if !res.insert_first_no_warn(args, pvs)? {
                        let targets = format!("{} and {}", source_pretty, target_pretty);
                        Self::not_avail_warn("pv", &targets, "progress bar", "sourcepv,targetpv");
                    }
                } else if enabled.contains("sourcepv") {
                    res.insert_all_checked(
                        args,
                        "progress bar",
                        [OptionalCommand {
                            cmd: source_pv,
                            insert_as: "sourcepv",
                            skip_with: "sourcepv",
                        }],
                    )?;
                } else if enabled.contains("targetpv") {
                    res.insert_all_checked(
                        args,
                        "progress bar",
                        [OptionalCommand {
                            cmd: target_pv,
                            insert_as: "targetpv",
                            skip_with: "targetpv",
                        }],
                    )?;
                }
                if enabled.contains("compress") {
                    res.insert_all_checked(
                        args,
                        "compression",
                        [
                            OptionalCommand {
                                cmd: source_compress.unwrap(),
                                insert_as: "sourcecompress",
                                skip_with: "compress",
                            },
                            OptionalCommand {
                                cmd: target_decompress.unwrap(),
                                insert_as: "targetcompress",
                                skip_with: "compress",
                            },
                        ],
                    )?;
                }
                if enabled.contains("sourcembuffer") {
                    res.insert_all_checked(
                        args,
                        "buffering and bandwith limits",
                        [OptionalCommand {
                            cmd: source_mbuffer,
                            insert_as: "sourcembuffer",
                            skip_with: "sourcembuffer",
                        }],
                    )?;
                }
                if enabled.contains("targetmbuffer") {
                    res.insert_all_checked(
                        args,
                        "buffering and write limits",
                        [OptionalCommand {
                            cmd: target_mbuffer,
                            insert_as: "targetmbuffer",
                            skip_with: "targetmbuffer",
                        }],
                    )?;
                }
            }
            ConnectionType::RemoteIndirect => {
                // "sourcepv", "sourcecompress", "targetmbuffer", "localmbuffer", "localcompress", "localpv", //"localcompress", //"localmbuffer", "targetcompress"
                // Decide compress first
                let compress = if enabled.contains("compress") {
                    res.insert_all_checked(
                        args,
                        "compression",
                        [
                            OptionalCommand {
                                cmd: source_compress.unwrap(),
                                insert_as: "sourcecompress",
                                skip_with: "compress",
                            },
                            OptionalCommand {
                                cmd: target_decompress.unwrap(),
                                insert_as: "targetcompress",
                                skip_with: "compress",
                            },
                        ],
                    )?
                } else {
                    false
                };
                // Now try local pv
                let mut pv_warn = Vec::new();
                let local_pv = if compress {
                    if enabled.contains("localpv") && enabled.contains("localcompress") {
                        res.insert_all_delay_warn(
                            args,
                            [
                                OptionalCommand {
                                    cmd: local_pv,
                                    insert_as: "localpv",
                                    skip_with: "localpv",
                                },
                                OptionalCommand {
                                    cmd: local_compress.unwrap(),
                                    insert_as: "localcompress",
                                    skip_with: "localcompress",
                                },
                                OptionalCommand {
                                    cmd: local_decompress.unwrap(),
                                    insert_as: "localdecompress",
                                    skip_with: "localcompress",
                                },
                            ],
                            &mut pv_warn,
                        )?
                    } else {
                        false
                    }
                } else if enabled.contains("localpv") {
                    res.insert_all_delay_warn(
                        args,
                        [OptionalCommand {
                            cmd: local_pv,
                            insert_as: "localpv",
                            skip_with: "localpv",
                        }],
                        &mut pv_warn,
                    )?
                } else {
                    false
                };
                // If local_pv failed/is disabled try sourcepv
                if !local_pv && enabled.contains("sourcepv") {
                    res.insert_all_delay_warn(
                        args,
                        [OptionalCommand {
                            cmd: source_pv,
                            insert_as: "sourcepv",
                            skip_with: "sourcepv",
                        }],
                        &mut pv_warn,
                    )?;
                }
                // simpler to just print everything separately
                for (cmd, target, skip_with) in pv_warn {
                    Self::not_avail_warn(cmd, target, "progress bar", skip_with);
                }
                if enabled.contains("sourcembuffer") {
                    res.insert_all_checked(
                        args,
                        "buffering and bandwith limits",
                        [OptionalCommand {
                            cmd: source_mbuffer,
                            insert_as: "sourcembuffer",
                            skip_with: "sourcembuffer",
                        }],
                    )?;
                }
                if enabled.contains("targetmbuffer") {
                    res.insert_all_checked(
                        args,
                        "buffering and write limits",
                        [OptionalCommand {
                            cmd: target_mbuffer,
                            insert_as: "targetmbuffer",
                            skip_with: "targetmbuffer",
                        }],
                    )?;
                }
                if local_pv && enabled.contains("localmbuffer") {
                    // manual
                    if res.insert_all_checked(
                        args,
                        "local buffering and bandwidth limits",
                        [OptionalCommand {
                            cmd: local_source_mbuffer,
                            insert_as: "localsourcembuffer",
                            skip_with: "localmbuffer",
                        }],
                    )? {
                        res.inner.insert("localtargetmbuffer", local_target_mbuffer);
                    }
                }
            }
        };
        Ok(res)
    }

    fn get_pv<'cmd>(&self, pv_key: &str, pv_size_str: &'cmd str) -> Option<Cmd<'args>>
    where
        'args: 'cmd,
    {
        self.inner.get(pv_key).cloned().map(|mut pv| {
            if pv_size_str != "0" {
                pv.arg("-s");
                pv.arg(pv_size_str);
            }
            pv
        })
    }

    // We build one or two shell pipes, depending on whether the hosts are the same or not
    pub fn build_sync_pipelines<'cmd>(
        &self,
        send_cmd: Cmd<'args>,
        recv_cmd: Cmd<'args>,
        pv_size_str: &'cmd str,
    ) -> Pipelines<'args, 'cmd>
    where
        'args: 'cmd,
    {
        let source_pv = self.get_pv("sourcepv", pv_size_str);
        let target_pv = self.get_pv("targetpv", pv_size_str);
        let local_pv = self.get_pv("localpv", pv_size_str);
        let source_compress = self.inner.get("sourcecompress").cloned();
        let target_compress = self.inner.get("targetcompress").cloned();
        let local_compress = self.inner.get("localcompress").cloned();
        let local_decompress = self.inner.get("localdecompress").cloned();
        let source_mbuffer = self.inner.get("sourcembuffer").cloned();
        let target_mbuffer = self.inner.get("targetmbuffer").cloned();
        let local_source_mbuffer = self.inner.get("localsourcembuffer").cloned();
        let local_target_mbuffer = self.inner.get("localtargetmbuffer").cloned();

        match &self.conn_type {
            ConnectionType::Local => {
                //"localpv", "localmbuffer"
                let source_pipeline = [Some(send_cmd), source_pv, source_mbuffer, Some(recv_cmd)];
                let source_pipeline = Pipeline::from(
                    self.source_cmd_target,
                    source_pipeline.into_iter().flatten().collect(),
                )
                .expect("contains some");
                (source_pipeline, None, None)
            }
            ConnectionType::Push => {
                // "localpv", "localcompress", "localmbuffer", "targetcompress", "targetmbuffer"
                let source_terminal = source_pv.is_some();
                let source_pipeline = [Some(send_cmd), source_pv, source_compress, source_mbuffer];
                let mut source_pipeline = Pipeline::from(
                    self.source_cmd_target,
                    source_pipeline.into_iter().flatten().collect(),
                )
                .expect("contains some");
                source_pipeline.0.use_terminal_if_ssh(source_terminal);
                let target_pipeline = [target_compress, target_mbuffer, Some(recv_cmd)];
                let target_pipeline = Pipeline::from(
                    self.target_cmd_target,
                    target_pipeline.into_iter().flatten().collect(),
                );
                (source_pipeline, None, target_pipeline)
            }
            ConnectionType::Pull => {
                // "sourcecompress", "sourcembuffer", "localcompress", "localmbuffer", "localpv"
                let source_pipeline = [Some(send_cmd), source_compress, source_mbuffer];
                let source_pipeline = Pipeline::from(
                    self.source_cmd_target,
                    source_pipeline.into_iter().flatten().collect(),
                )
                .expect("contains some");
                let target_terminal = target_pv.is_some();
                let target_pipeline = [target_compress, target_mbuffer, target_pv, Some(recv_cmd)];
                let mut target_pipeline = Pipeline::from(
                    self.target_cmd_target,
                    target_pipeline.into_iter().flatten().collect(),
                );
                if let Some(target_pipeline) = target_pipeline.as_mut() {
                    target_pipeline.0.use_terminal_if_ssh(target_terminal)
                }
                (source_pipeline, None, target_pipeline)
            }
            ConnectionType::RemoteDirect => {
                // "sourcepv", "sourcecompress", "sourcembuffer", "targetcompress", "targetmbuffer", "targetpv"
                let source_terminal = source_pv.is_some();
                let source_pipeline = [Some(send_cmd), source_pv, source_compress, source_mbuffer];
                let mut source_pipeline = Pipeline::from(
                    self.source_cmd_target,
                    source_pipeline.into_iter().flatten().collect(),
                )
                .expect("contains some");
                source_pipeline.0.use_terminal_if_ssh(source_terminal);
                let target_terminal = target_pv.is_some();
                let target_pipeline = [target_compress, target_mbuffer, target_pv, Some(recv_cmd)];
                let mut target_pipeline = Pipeline::from(
                    self.target_cmd_target,
                    target_pipeline.into_iter().flatten().collect(),
                );
                if let Some(target_pipeline) = target_pipeline.as_mut() {
                    target_pipeline.0.use_terminal_if_ssh(target_terminal)
                }
                (source_pipeline, None, target_pipeline)
            }
            ConnectionType::RemoteIndirect => {
                // "sourcepv", "sourcecompress", "targetmbuffer", "localmbuffer", "localcompress", "localpv", //"localcompress", //"localmbuffer", "targetcompress"
                let source_terminal = source_pv.is_some();
                let source_pipeline = [Some(send_cmd), source_pv, source_compress, source_mbuffer];
                let mut source_pipeline = Pipeline::from(
                    self.source_cmd_target,
                    source_pipeline.into_iter().flatten().collect(),
                )
                .expect("contains some");
                source_pipeline.0.use_terminal_if_ssh(source_terminal);
                let local_pipeline = [
                    local_target_mbuffer,
                    local_decompress,
                    local_pv,
                    local_compress,
                    local_source_mbuffer,
                ];
                let local_pipeline = Pipeline::from(
                    self.local_cmd_target,
                    local_pipeline.into_iter().flatten().collect(),
                );
                let target_pipeline = [target_compress, target_mbuffer, Some(recv_cmd)];
                let target_pipeline = Pipeline::from(
                    self.target_cmd_target,
                    target_pipeline.into_iter().flatten().collect(),
                );
                (source_pipeline, local_pipeline, target_pipeline)
            }
        }
    }

    pub fn run_sync_pipelines<'cmd>(
        &self,
        (source_pipeline, local_pipeline, target_pipeline): Pipelines<'args, 'cmd>,
    ) -> io::Result<()> {
        // Set stdio and stderr
        debug!("source pipeline: {source_pipeline}");
        let mut source_cmd = source_pipeline.to_cmd();
        source_cmd.stderr(Stdio::inherit());
        source_cmd.stdin(Stdio::inherit()); // ssh does not like it if stdin is not a terminal
        let local_cmd = if let Some(local_pipeline) = local_pipeline {
            debug!("local pipeline: {local_pipeline}");
            let mut local_cmd = local_pipeline.to_cmd();
            local_cmd.stderr(Stdio::inherit());
            Some(local_cmd)
        } else {
            None
        };
        let target_cmd = if let Some(target_pipeline) = target_pipeline {
            debug!("target pipeline: {target_pipeline}");
            let mut target_cmd = target_pipeline.to_cmd();
            target_cmd.stderr(Stdio::inherit());
            Some(target_cmd)
        } else {
            None
        };
        // Build stdout pipes and run
        let output = match (local_cmd, target_cmd) {
            (_, None) => source_cmd.stdout(Stdio::inherit()).output()?,
            (None, Some(mut target_cmd)) => {
                source_cmd.stdout(Stdio::piped());
                let mut source_process = source_cmd.spawn()?;
                let source_stdout = source_process.stdout.take().expect("stdout is piped");
                let _source_process = AutoTerminate::new(source_process);
                target_cmd
                    .stdin(Stdio::from(source_stdout))
                    .stdout(Stdio::inherit())
                    .output()?
            }
            (Some(mut local_cmd), Some(mut target_cmd)) => {
                source_cmd.stdout(Stdio::piped());
                let mut source_process = source_cmd.spawn()?;
                let source_stdout = source_process.stdout.take().expect("stdout is piped");
                let _source_process = AutoTerminate::new(source_process);
                local_cmd.stdin(Stdio::from(source_stdout));
                local_cmd.stdout(Stdio::piped());
                let mut local_process = local_cmd.spawn()?;
                let local_stdout = local_process.stdout.take().expect("stdout is piped");
                let _local_process = AutoTerminate::new(local_process);
                target_cmd
                    .stdin(Stdio::from(local_stdout))
                    .stdout(Stdio::inherit())
                    .output()?
            }
        };
        if !output.status.success() {
            return Err(io::Error::other("sync pipeline failed"));
        };
        Ok(())
    }
}
