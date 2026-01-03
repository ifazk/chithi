//  Chobi and Chithi: Managment tools for ZFS snapshot, send, and recv
//  Copyright (C) 2025  Ifaz Kabir

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

use log::{debug, error};
use std::{
    borrow::Cow,
    fmt::Display,
    io,
    process::{Command, ExitStatus, Output, Stdio},
};

use crate::chithi::sys;

type SshOption = String;

#[derive(PartialEq, Eq)]
pub struct Ssh<'args> {
    host: &'args str,
    cipher: Option<&'args str>,
    config: Option<&'args str>,
    identity: Option<&'args str>,
    port: Option<&'args str>,
    control: Option<String>,
    options: &'args Vec<SshOption>,
}

impl<'args> Ssh<'args> {
    pub fn new(
        host: &'args str,
        cipher: Option<&'args str>,
        config: Option<&'args str>,
        identity: Option<&'args str>,
        port: Option<&'args str>,
        options: &'args Vec<SshOption>,
    ) -> Self {
        Self {
            host,
            cipher,
            config,
            identity,
            port,
            control: None,
            options,
        }
    }
    fn make_pre_cmd(&self) -> Command {
        let mut cmd = Command::new("ssh");
        if let Some(cipher) = self.cipher {
            cmd.args(["-c", cipher]);
        };
        if let Some(config) = self.config {
            cmd.args(["-F", config]);
        };
        if let Some(identity) = self.identity {
            cmd.args(["-i", identity]);
        };
        if let Some(port) = self.port {
            cmd.args(["-p", port]);
        };
        for option in self.options {
            cmd.args(["-o", option]);
        }
        cmd
    }
    pub fn to_cmd(&self) -> Command {
        let mut cmd = self.make_pre_cmd();
        if let Some(control) = &self.control {
            cmd.args(["-S", control]);
        }
        cmd.arg(self.host);
        cmd
    }
}

#[derive(PartialEq, Eq)]
pub enum CmdTarget<'args> {
    Local,
    Remote { ssh: Ssh<'args> },
}

impl<'args> CmdTarget<'args> {
    pub fn new_local() -> Self {
        Self::Local
    }
    pub fn new(
        host: Option<&'args str>,
        cipher: Option<&'args str>,
        config: Option<&'args str>,
        identity: Option<&'args str>,
        port: Option<&'args str>,
        ssh_options: &'args Vec<SshOption>,
    ) -> Self {
        host.map_or(Self::Local, |host| {
            let ssh = Ssh::new(host, cipher, config, identity, port, ssh_options);
            Self::Remote { ssh }
        })
    }
    pub fn is_remote(&self) -> bool {
        match self {
            CmdTarget::Local => false,
            CmdTarget::Remote { .. } => true,
        }
    }
    fn make_check(&self, base: &'static str) -> Command {
        // Like syncoid, use POSIX compatible command to check for program existence
        // TODO figure out if there's a RUST native way of doing this
        match self {
            CmdTarget::Local => {
                debug!("checking local command {base}");
                let mut cmd = Command::new("sh");
                cmd.arg("-c");
                cmd.arg(format!("command -v {base}"));
                cmd
            }
            CmdTarget::Remote { ssh } => {
                debug!("checking remote command {base} in {}", ssh.host);
                let mut cmd = ssh.to_cmd();
                cmd.args(["command", "-v", base]);
                cmd
            }
        }
    }
    fn make_cmd(&self, base: &'static str) -> Command {
        match self {
            CmdTarget::Local => Command::new(base),
            CmdTarget::Remote { ssh } => {
                let mut cmd = ssh.to_cmd();
                cmd.arg(base);
                cmd
            }
        }
    }
    pub fn set_control(&mut self, control: Option<&str>) {
        match self {
            CmdTarget::Local => {}
            CmdTarget::Remote { ssh } => ssh.control = control.map(|c| c.to_string()),
        }
    }
    pub fn make_control(&mut self) -> io::Result<Option<&str>> {
        // Syncoid does sshcmd = sshcmd $args{sshconfig} $args{sshcipher} $sshoptions $args{sshport} $args{sshkey}
        // Then runs sshcmd -M -S socket -o ControlPersist=1m $args{sshport} $rhost exit
        match self {
            CmdTarget::Local => Ok(None),
            CmdTarget::Remote { ssh } => {
                let host_sanitized: String = ssh
                    .host
                    .chars()
                    .map(|c| if c == '@' { '-' } else { c })
                    .filter(|&c| c.is_ascii_alphanumeric() || c == '-')
                    .take(50)
                    .collect();
                let (year, mon, mday, hour, min, sec) = {
                    use chrono::{Datelike, Timelike};
                    let local = chrono::Local::now();
                    let year = local.year();
                    let mon = local.month();
                    let mday = local.day();
                    let hour = local.hour();
                    let min = local.minute();
                    let sec = local.second();
                    (year, mon, mday, hour, min, sec)
                };
                let id = std::process::id();
                let rand = rand::random_range(0..1000u32);
                let control = format!(
                    "/tmp/chithi-{host_sanitized}-{year:04}{mon:02}{mday:02}{hour:02}{min:02}{sec:02}-{id}-{rand}"
                );
                debug!(
                    "creating ssh master control socket for {}: {control}",
                    ssh.host
                );
                let mut cmd = ssh.make_pre_cmd();
                // TODO ssh port?
                cmd.args([
                    "-M",
                    "-S",
                    &control,
                    "-o",
                    "ControlPersist=1m",
                    ssh.host,
                    "exit",
                ]);
                let err = io::Error::other("creating master control failed");
                match cmd.status() {
                    Ok(exit) if exit.success() => {
                        let mut echo_test = ssh.make_pre_cmd();
                        echo_test.args(["-S", &control, ssh.host, "echo", "-n"]);
                        match echo_test.status() {
                            Ok(exit) if exit.success() => {
                                ssh.control = Some(control);
                                Ok(ssh.control.as_deref())
                            }
                            Ok(exit) => {
                                error!("master control echo test exited with {exit}");
                                Err(err)
                            }
                            Err(e) => {
                                error!("master control echo test failed with {e}");
                                Err(err)
                            }
                        }
                    }
                    Ok(exit) => {
                        error!("creating master control exited with {exit}");
                        Err(err)
                    }
                    Err(e) => {
                        error!("creating master control failed with {e}");
                        Err(err)
                    }
                }
            }
        }
    }

    pub fn destroy_control(&mut self) -> io::Result<()> {
        match self {
            CmdTarget::Local => Ok(()),
            CmdTarget::Remote { ssh } => match ssh.control.take() {
                Some(control) => {
                    let mut exit_cmd = ssh.make_pre_cmd();
                    exit_cmd.args(["-S", control.as_str(), ssh.host, "-O", "exit"]);
                    let status = exit_cmd
                        .stdout(Stdio::null())
                        .stdin(Stdio::null())
                        .stderr(Stdio::null())
                        .status()?;
                    if status.success() {
                        Ok(())
                    } else {
                        Err(io::Error::other("destroying ssh control failed"))
                    }
                }
                None => Ok(()),
            },
        }
    }
    pub fn on_str(&self) -> &str {
        match self {
            CmdTarget::Local => "",
            CmdTarget::Remote { .. } => " on ",
        }
    }
    pub fn host(&self) -> &str {
        match self {
            CmdTarget::Local => "",
            CmdTarget::Remote { ssh } => ssh.host,
        }
    }
    pub fn pretty_str(&self) -> &'args str {
        match self {
            CmdTarget::Local => "local machine",
            CmdTarget::Remote { ssh } => ssh.host,
        }
    }
}

impl<'args> Display for CmdTarget<'args> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmdTarget::Local => {}
            CmdTarget::Remote { ssh } => {
                write!(f, "ssh ")?;
                if let Some(cipher) = ssh.cipher {
                    write!(f, "-c {cipher} ")?;
                };
                if let Some(config) = ssh.config {
                    write!(f, "-F {config} ")?;
                };
                if let Some(identity) = ssh.identity {
                    write!(f, "-i {identity} ")?;
                };
                if let Some(port) = ssh.port {
                    write!(f, "-p {port} ")?;
                };
                if let Some(control) = &ssh.control {
                    write!(f, "-S {} ", control)?;
                }
                for option in ssh.options {
                    write!(f, "-o {} ", option)?;
                }
                write!(f, "{} ", ssh.host)?;
            }
        };
        Ok(())
    }
}

pub struct Cmd<'args, T> {
    target: &'args CmdTarget<'args>,
    sudo: bool,
    base: &'static str,
    args: T,
}

impl<'args, T> Cmd<'args, T> {
    pub fn to_local(self) -> Self {
        Self {
            target: &CmdTarget::Local,
            sudo: self.sudo,
            base: self.base,
            args: self.args,
        }
    }
    pub fn target(&self) -> &'args CmdTarget<'args> {
        self.target
    }
    pub fn base(&self) -> &'static str {
        self.base
    }
}

fn escape_str<'a>(s: &'a str) -> Cow<'a, str> {
    if s.contains([
        '#', '\'', '"', ' ', '\t', '\n', '\r', '|', '&', ';', '<', '>', '(', ')', '$', '*', '?',
        '[', ']', '^', '!', '~', '%', '{', '}', //'=', ',', '-',
    ]) {
        let mut result = String::new();
        result.push('\''); // start quote
        for ch in s.chars() {
            if ch == '\'' {
                result.push('\''); // end quote
                result.push_str("\\'"); // single quote that's escaped
                result.push('\''); // restart quote
            } else {
                result.push(ch);
            }
        }
        result.push('\''); // end quote
        Cow::Owned(result)
    } else {
        Cow::Borrowed(s)
    }
}

impl<'args, 'cmd, T: AsRef<[&'cmd str]>> Cmd<'args, T> {
    pub fn new(target: &'args CmdTarget<'args>, sudo: bool, cmd: &'static str, args: T) -> Self {
        Self {
            target,
            sudo,
            base: cmd,
            args,
        }
    }

    pub fn to_cmd(&self) -> Command {
        let mut cmd = if self.sudo {
            let mut cmd = self.target.make_cmd("sudo");
            cmd.arg(self.base);
            cmd
        } else {
            self.target.make_cmd(self.base)
        };
        if self.target.is_remote() {
            for &arg in self.args.as_ref() {
                let escaped_arg = escape_str(arg);
                let arg: &str = &escaped_arg;
                cmd.arg(arg);
            }
            return cmd;
        }
        for arg in self.args.as_ref() {
            cmd.arg(arg);
        }
        cmd
    }

    pub fn status(&self, debug: bool) -> io::Result<ExitStatus> {
        let mut cmd = self.to_cmd();
        cmd.stdin(Stdio::null()).stdout(Stdio::null());
        if debug {
            cmd.stderr(Stdio::inherit());
        } else {
            cmd.stderr(Stdio::null());
        };
        cmd.status()
    }

    /// Captures the stdout of the command. Stderr is inherited if debug is
    /// true, and nulled otherwise.
    pub fn output(&self, debug: bool) -> io::Result<Output> {
        let mut cmd = self.to_cmd();
        cmd.stdin(Stdio::null()).stdout(Stdio::piped());
        if debug {
            cmd.stderr(Stdio::inherit());
        } else {
            cmd.stderr(Stdio::null());
        };
        cmd.output()
    }

    pub fn to_mut(&self) -> Cmd<'args, Vec<&'cmd str>> {
        self.into()
    }

    pub fn to_check(&self) -> Command {
        self.target.make_check(self.base)
    }

    pub fn check_exists(&self) -> io::Result<()> {
        let exists = self.to_check().output()?.status.success();
        if !exists {
            error!(
                "{} does not exist in {}",
                self.base,
                self.target.pretty_str()
            );
            return Err(io::Error::new(io::ErrorKind::NotFound, "command not found"));
        }
        Ok(())
    }

    /// Run command printing and catputuring output (stdout and stderr)
    pub fn capture(&self) -> io::Result<Output> {
        let mut command = self.to_cmd();
        sys::capture(&mut command)
    }

    /// Run command inheriting stderr and capturing std output
    pub fn capture_stdout(&self) -> io::Result<Output> {
        let mut command = self.to_cmd();
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        command.output()
    }
}

impl<'args, 'cmd> Cmd<'args, Vec<&'cmd str>> {
    pub fn arg(&mut self, value: &'cmd str) {
        self.args.push(value);
    }
    pub fn args<T: AsRef<[&'cmd str]>>(&mut self, values: T) {
        for &value in values.as_ref() {
            self.args.push(value);
        }
    }
}

impl<'args, 'cmd, T: AsRef<[&'cmd str]>> Display for Cmd<'args, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sudo = if self.sudo { "sudo " } else { "" };
        write!(f, "{}{}{}", self.target, sudo, self.base)?;
        if self.target.is_remote() {
            for &arg in self.args.as_ref() {
                write!(f, " {}", escape_str(arg))?;
            }
            return Ok(());
        }
        for &arg in self.args.as_ref() {
            write!(f, " {}", arg)?;
        }
        Ok(())
    }
}

impl<'args, 'cmd, T: AsRef<[&'cmd str]>> From<&Cmd<'args, T>> for Cmd<'args, Vec<&'cmd str>> {
    fn from(
        Cmd {
            target,
            sudo,
            base,
            args,
        }: &Cmd<'args, T>,
    ) -> Self {
        Self {
            target,
            sudo: *sudo,
            base,
            args: args.as_ref().to_vec(),
        }
    }
}

/// Builds a vector of commands that will be passed as a script via ssh or sh
/// -c. Used by Pipeline and Sequence.
pub struct CmdVec<'args, T> {
    target: &'args CmdTarget<'args>,
    use_terminal_if_ssh: bool,
    cmds: Vec<Cmd<'args, T>>,
}

impl<'args, T> CmdVec<'args, T> {
    pub fn new(target: &'args CmdTarget<'args>, cmd: Cmd<'args, T>) -> Self {
        Self {
            target,
            use_terminal_if_ssh: false,
            cmds: vec![cmd],
        }
    }
    pub fn add_cmd(&mut self, cmd: Cmd<'args, T>) {
        self.cmds.push(cmd);
    }
    pub fn is_remote(&self) -> bool {
        self.target.is_remote()
    }
    pub fn use_terminal_if_ssh(&mut self, value: bool) {
        self.use_terminal_if_ssh = value;
    }
    /// return none if input is empty, otherwise guaranteed to be some
    pub fn from(target: &'args CmdTarget<'args>, mut from: Vec<Cmd<'args, T>>) -> Option<Self> {
        from.reverse();
        if let Some(first) = from.pop() {
            let mut pipeline = Self::new(target, first);
            while let Some(other) = from.pop() {
                pipeline.add_cmd(other);
            }
            Some(pipeline)
        } else {
            None
        }
    }
}

impl<'args, 'cmd, T: AsRef<[&'cmd str]>> CmdVec<'args, T> {
    fn to_cmd_with_sep(&self, sep: &str) -> Command {
        match self.target {
            CmdTarget::Local => {
                // take a shortcut if there's only one cmd
                if self.cmds.len() == 1 {
                    return self.cmds[0].to_cmd();
                }
                let mut cmd = Command::new("sh");
                cmd.args(["-c", "--"]);
                if let Some(inner) = self.cmds.first() {
                    use std::fmt::Write;
                    let mut arg = String::new();
                    write!(arg, "{}", Self::escape_cmd(inner)).expect("formatting should not fail");
                    for inner in &self.cmds[1..] {
                        write!(arg, " {} {}", sep, Self::escape_cmd(inner))
                            .expect("formatting should not fail");
                    }
                    cmd.arg(arg);
                };
                cmd
            }
            CmdTarget::Remote { ssh } => {
                let mut cmd = Command::new("ssh");
                if self.use_terminal_if_ssh {
                    cmd.args(["-t", "-o", "LogLevel=QUIET"]);
                }
                if let Some(control) = &ssh.control {
                    cmd.args(["-S", control]);
                }
                for option in ssh.options {
                    cmd.args(["-o", option]);
                }
                cmd.arg(ssh.host);
                // We don't have control over what shell is interpreting these
                // bytes. Zsh really doesn't like foo#bar, so let's escape those
                // and anthing that contains special shell characters.
                if let Some(first) = self.cmds.first() {
                    cmd.arg(Self::escape_cmd(first));
                    for other in &self.cmds[1..] {
                        cmd.arg(sep);
                        cmd.arg(Self::escape_cmd(other));
                    }
                };
                cmd
            }
        }
    }

    fn escape_cmd(cmd: &Cmd<'args, T>) -> String {
        let mut result = String::new();
        if cmd.target.is_remote() {
            let ssh = format!("{}", cmd.target);
            result.push_str(&ssh);
        }
        if cmd.sudo {
            result.push_str("sudo ");
        }
        result.push_str(cmd.base);
        if cmd.target.is_remote() {
            // potentially needs double escape
            result.push(' ');
            for &arg in cmd.args.as_ref() {
                result.push(' ');
                let arg = escape_str(arg);
                result.push_str(&escape_str(&arg));
            }
            return result;
        }
        for &arg in cmd.args.as_ref() {
            result.push(' ');
            result.push_str(&escape_str(arg));
        }
        result
    }

    fn fmt_with_sep(&self, f: &mut std::fmt::Formatter<'_>, sep: &str) -> std::fmt::Result {
        match self.target {
            CmdTarget::Local => {
                // take a shortcut if there's only one cmd
                if self.cmds.len() == 1 {
                    write!(f, "{}", self.cmds[0])?;
                    return Ok(());
                }
                write!(f, "sh -c -- ")?;
                if let Some(cmd) = self.cmds.first() {
                    write!(f, "{}", cmd)?;
                    for cmd in &self.cmds[1..] {
                        write!(f, " {} {}", sep, cmd)?;
                    }
                }
            }
            CmdTarget::Remote { .. } => {
                write!(f, "{}", self.target)?;
                if let Some(cmd) = self.cmds.first() {
                    write!(f, "{}", Self::escape_cmd(cmd))?;
                    for cmd in &self.cmds[1..] {
                        write!(f, " {} {}", sep, Self::escape_cmd(cmd))?;
                    }
                }
            }
        }
        Ok(())
    }
}

/// Thin wrapper around CmdVec to build a pipeline of commands that will be
/// passed as a script via ssh or sh -c
pub struct Pipeline<'args, T>(pub CmdVec<'args, T>);

impl<'args, T> Pipeline<'args, T> {
    pub fn new(target: &'args CmdTarget<'args>, cmd: Cmd<'args, T>) -> Self {
        Self(CmdVec::new(target, cmd))
    }
    pub fn from(target: &'args CmdTarget<'args>, from: Vec<Cmd<'args, T>>) -> Option<Self> {
        CmdVec::from(target, from).map(Self)
    }
}

impl<'args, 'cmd, T: AsRef<[&'cmd str]>> Pipeline<'args, T> {
    pub fn to_cmd(&self) -> Command {
        self.0.to_cmd_with_sep("|")
    }
}

impl<'args, 'cmd, T: AsRef<[&'cmd str]>> Display for Pipeline<'args, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt_with_sep(f, "|")
    }
}

/// Thin wrapper around CmdVec to build a pipeline of commands that will be
/// passed as a script via ssh or sh -c
pub struct Sequence<'args, T>(pub CmdVec<'args, T>);

impl<'args, T> Sequence<'args, T> {
    pub fn new(target: &'args CmdTarget<'args>, cmd: Cmd<'args, T>) -> Self {
        Self(CmdVec::new(target, cmd))
    }
    pub fn from(target: &'args CmdTarget<'args>, from: Vec<Cmd<'args, T>>) -> Option<Self> {
        CmdVec::from(target, from).map(Self)
    }
}

impl<'args, 'cmd, T: AsRef<[&'cmd str]>> Sequence<'args, T> {
    pub fn to_cmd(&self) -> Command {
        self.0.to_cmd_with_sep(";")
    }

    pub fn status(&self, debug: bool) -> io::Result<ExitStatus> {
        let mut cmd = self.to_cmd();
        cmd.stdin(Stdio::null()).stdout(Stdio::null());
        if debug {
            cmd.stderr(Stdio::inherit());
        } else {
            cmd.stderr(Stdio::null());
        };
        cmd.status()
    }
}

impl<'args, 'cmd, T: AsRef<[&'cmd str]>> Display for Sequence<'args, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt_with_sep(f, ";")
    }
}
