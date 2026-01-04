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

use libc::getuid;
use std::{borrow::Cow, fmt::Display, io};

/// For syncing filesystems/datasets have a notion of being a source dataset or a target
#[derive(Debug, Clone, Copy)]
pub enum Role {
    Source,
    Target,
}

/// Check whether we should assume operations are as root
pub fn get_is_roots(
    source: Option<&str>,
    target: Option<&str>,
    bypass_root_check: bool,
) -> (bool, bool) {
    fn get_is_root(host: Option<&str>, bypass_root_check: bool) -> Option<bool> {
        host.and_then(|user| user.split_once('@'))
            .map(|(user, _)| bypass_root_check || user == "root")
    }
    let source_is_root = get_is_root(source, bypass_root_check);
    let target_is_root = get_is_root(target, bypass_root_check);
    match (source_is_root, target_is_root) {
        (Some(s), Some(t)) => (s, t),
        (s, t) => {
            let local_is_root = unsafe { getuid() == 0 };
            (s.unwrap_or(local_is_root), t.unwrap_or(local_is_root))
        }
    }
}

pub struct Fs<'args> {
    pub host: Option<&'args str>,
    pub fs: Cow<'args, str>, // use owned for child datasets
    pub role: Role,
    pub origin: Option<String>,
}

fn split_host_at_colon(host: &str) -> Option<(&str, &str)> {
    let mut iter = host.char_indices();
    while let Some((pos, c)) = iter.next() {
        if c == '/' {
            return None;
        }
        if c == ':' {
            return Some((&host[0..pos], iter.as_str()));
        }
    }
    None
}

impl<'args> Fs<'args> {
    pub fn new(host_opt: Option<&'args str>, fs: &'args str, role: Role) -> Self {
        // There are three cases
        // 1. There's a separately provided hostname (which can also contain a
        // username), in which case we .
        // This provided hostname can be the empty string, see below.
        // 2. There's no seprately provided hostname, and there's a : in fs
        // before any '/' -> host:filesystem, user@host:filesystem, or
        // user@host:pool/filesystem
        // 3. If there's no seprately provided hostname, and there's no : in fs,
        // then fs is treated as a local filesystem
        //
        // Syncoid tries to figure out if : is part of a local pool name or if
        // it is used to separate the hostname from the filesystem, but we
        // don't. If there is a : in the poolname, then hostname must be set
        // separately.
        let (host, fs) = match host_opt {
            Some(host) => (if host.is_empty() { None } else { Some(host) }, fs),
            None => {
                if let Some((host, fs)) = split_host_at_colon(fs) {
                    (Some(host), fs)
                } else {
                    (None, fs)
                }
            }
        };
        Self {
            host,
            fs: fs.into(),
            role,
            origin: None,
        }
    }
    /// Creates a child dataset. Origin can be "-".
    pub fn new_child(&self, name: String, origin: String) -> Self {
        Self {
            host: self.host,
            fs: name.into(),
            role: self.role,
            origin: if origin == "-" { None } else { Some(origin) },
        }
    }
    pub fn child_from_source(
        &self,
        source: &Self,
        child: &Self,
        clone_handling: bool,
    ) -> io::Result<Self> {
        let Some(dataset) = child.fs.strip_prefix(source.fs.as_ref()) else {
            return Err(io::Error::other(format!(
                "child {child} did not start with source {source}"
            )));
        };
        let mut target_dataset = self.fs.to_string();
        target_dataset.push_str(dataset);
        let target_origin = clone_handling
            .then(|| {
                child
                    .origin
                    .as_ref()
                    .and_then(|child_origin| child_origin.strip_prefix(source.fs.as_ref()))
                    .map(|target_origin_dataset_snapshot| {
                        let mut target_origin_full_snapshot = self.fs.to_string();
                        target_origin_full_snapshot.push_str(target_origin_dataset_snapshot);
                        target_origin_full_snapshot
                    })
            })
            .flatten()
            .unwrap_or_else(|| "-".to_string());
        Ok(self.new_child(target_dataset, target_origin))
    }
    pub fn origin_dataset(&self) -> Option<&str> {
        self.origin
            .as_deref()
            .and_then(|s| s.split_once('@').map(|split| split.0))
    }
}

impl<'args> Display for Fs<'args> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fs)?;
        if let Some(host) = self.host {
            write!(f, " on {}", host)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_user_hosts() {
        let Fs {
            host,
            fs,
            role: _,
            origin: _,
        } = Fs::new(None, "user@host:pool", Role::Source);
        assert_eq!(host, Some("user@host"));
        assert_eq!(fs, "pool");
        let Fs {
            host,
            fs,
            role: _,
            origin: _,
        } = Fs::new(None, "user@host:pool/filesystem", Role::Source);
        assert_eq!(host, Some("user@host"));
        assert_eq!(fs, "pool/filesystem");
    }

    #[test]
    fn simple_hosts_without_users() {
        let Fs {
            host,
            fs,
            role: _,
            origin: _,
        } = Fs::new(None, "host:pool", Role::Source);
        assert_eq!(host, Some("host"));
        assert_eq!(fs, "pool");
        let Fs {
            host,
            fs,
            role: _,
            origin: _,
        } = Fs::new(None, "host:pool/filesystem", Role::Source);
        assert_eq!(host, Some("host"));
        assert_eq!(fs, "pool/filesystem");
        let Fs {
            host,
            fs,
            role: _,
            origin: _,
        } = Fs::new(None, "host:pool/filesystem:alsofs", Role::Source);
        assert_eq!(host, Some("host"));
        assert_eq!(fs, "pool/filesystem:alsofs");
    }

    #[test]
    fn simple_user_hosts_pool_fs_colon() {
        let Fs {
            host,
            fs,
            role: _,
            origin: _,
        } = Fs::new(None, "user@host:pool:alsopool", Role::Source);
        assert_eq!(host, Some("user@host"));
        assert_eq!(fs, "pool:alsopool");
        let Fs {
            host,
            fs,
            role: _,
            origin: _,
        } = Fs::new(
            None,
            "user@host:pool:alsopool/filesystem:alsofs",
            Role::Source,
        );
        assert_eq!(host, Some("user@host"));
        assert_eq!(fs, "pool:alsopool/filesystem:alsofs");
    }

    #[test]
    fn empty_user_hosts() {
        let Fs {
            host,
            fs,
            role: _,
            origin: _,
        } = Fs::new(Some(""), "pool", Role::Source);
        assert_eq!(host, None);
        assert_eq!(fs, "pool");
        let Fs {
            host,
            fs,
            role: _,
            origin: _,
        } = Fs::new(Some(""), "pool/filesystem", Role::Source);
        assert_eq!(host, None);
        assert_eq!(fs, "pool/filesystem");
    }

    #[test]
    fn empty_user_hosts_pool_fs_colon() {
        let Fs {
            host,
            fs,
            role: _,
            origin: _,
        } = Fs::new(Some(""), "poolnothost:alsopool", Role::Source);
        assert_eq!(host, None);
        assert_eq!(fs, "poolnothost:alsopool");
        let Fs {
            host,
            fs,
            role: _,
            origin: _,
        } = Fs::new(
            Some(""),
            "poolnothost:alsopool/filesystem:alsofs",
            Role::Source,
        );
        assert_eq!(host, None);
        assert_eq!(fs, "poolnothost:alsopool/filesystem:alsofs");
    }

    #[test]
    fn nonempty_user_hosts_pool_fs_colon() {
        let Fs {
            host,
            fs,
            role: _,
            origin: _,
        } = Fs::new(Some("user@host"), "poolnothost:alsopool", Role::Source);
        assert_eq!(host, Some("user@host"));
        assert_eq!(fs, "poolnothost:alsopool");
        let Fs {
            host,
            fs,
            role: _,
            origin: _,
        } = Fs::new(
            Some("user@host"),
            "poolnothost:alsopool/filesystem:alsofs",
            Role::Source,
        );
        assert_eq!(host, Some("user@host"));
        assert_eq!(fs, "poolnothost:alsopool/filesystem:alsofs");
        let Fs {
            host,
            fs,
            role: _,
            origin: _,
        } = Fs::new(
            Some("user:wierduser@host:wierdhost"),
            "poolnothost:alsopool/filesystem:alsofs",
            Role::Source,
        );
        assert_eq!(host, Some("user:wierduser@host:wierdhost"));
        assert_eq!(fs, "poolnothost:alsopool/filesystem:alsofs");
    }
}
