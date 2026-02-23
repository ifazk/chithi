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
use log::info;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Display,
    io,
    ops::Deref,
};

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
    pub fn strip_parent_from<'a>(&self, child: &'a str) -> Option<&'a str> {
        child
            .strip_prefix(self.fs.as_ref())
            .map(|without_prefix| without_prefix.strip_prefix('/').unwrap_or(without_prefix))
    }
    /// self.fs shouldn't have leading or trailing /, and neither should child_datasets.
    /// child datasets should also not have double / anywhere.
    /// child datasets should have prefix self.fs.
    /// Returns:
    /// 1. the topologically sorted indices of child_datasets
    /// 2. any datasets that are excluded but needs to exist for successful cloning
    pub fn topological_sort<'a, 'b: 'a>(
        &self, // parent
        child_datasets: &'a [Fs<'b>],
    ) -> (Vec<usize>, HashSet<&'a str>) {
        #[derive(Debug)]
        struct Trie<'a> {
            index: Option<usize>,
            dataset: &'a str,
            children: HashMap<&'a str, Box<Self>>,
        }
        impl<'a> Trie<'a> {
            fn get_datasets_and_components<'c>(
                parent: &str,
                child: &'c str,
            ) -> Vec<(&'c str, &'c str)> {
                if parent == child {
                    return vec![(child, "")];
                }
                // we expect child datasets to have parent prefix
                let mut after_prefix = parent.len();
                // removing prefix might leave a leading /
                let bytes = child.as_bytes();
                // don't need length check because we returned early if parent
                // equals child and we expect child to have parent as prefix
                if bytes[after_prefix] == b'/' {
                    after_prefix += 1;
                };
                let mut res = Vec::new();
                let mut component_start = after_prefix;
                for idx in after_prefix..child.len() {
                    if bytes[idx] == b'/' {
                        res.push((&child[..idx], &child[component_start..idx]));
                        component_start = idx + 1;
                    }
                }
                res.push((child, &child[component_start..]));
                res
            }
            /// Inserts and returns the node
            fn insert_str(&mut self, parent: &str, child_dataset: &'a str) -> &mut Self {
                let mut res = self;
                for child in Self::get_datasets_and_components(parent, child_dataset) {
                    res = res.get_or_insert(child)
                }
                res
            }
            fn get_or_insert<'b>(&'b mut self, child: (&'a str, &'a str)) -> &'b mut Self {
                let (dataset, component) = child;
                if component.is_empty() {
                    return self;
                }
                self.children.entry(component).or_insert_with(|| {
                    Box::new(Self {
                        index: None,
                        dataset,
                        children: HashMap::new(),
                    })
                })
            }
            fn get_str<'b>(&'b self, parent: &str, child: &str) -> Option<&'b Self> {
                let mut current = self;
                for (_child, component) in Self::get_datasets_and_components(parent, child) {
                    if let Some(next) = current.children.get(component) {
                        current = next
                    } else {
                        return None;
                    }
                }
                Some(current)
            }
        }
        let mut root = if child_datasets.is_empty() {
            return (Vec::new(), HashSet::new());
        } else {
            // borrow from first element to get correct lifetime
            // we expect child to have parent as prefix
            let dataset = &child_datasets[0].fs.deref()[..self.fs.len()];
            Trie {
                index: None,
                dataset,
                children: HashMap::new(),
            }
        };
        // Insert child datasets
        for (idx, dataset) in child_datasets.iter().enumerate() {
            root.insert_str(&self.fs, &dataset.fs).index = Some(idx)
        }
        // Keep track of datasets that must exist
        let mut must_exist: HashSet<&'a str> = HashSet::new();
        // Build graph for topological sort
        let graph = {
            let mut graph = vec![HashSet::new(); child_datasets.len()];
            // DFS trie to add parent-child dependency edges
            let mut stack = Vec::new();
            stack.push((&root, 0usize));
            let mut parents: Vec<(usize, usize)> = Vec::new();
            while let Some((node, d)) = stack.pop() {
                // remove siblings and their desendents from parents stack
                while parents.last().is_some_and(|(_, sibling_d)| *sibling_d >= d) {
                    parents.pop();
                }
                // process this node
                if let Some(((from, _), to)) = parents.last().zip(node.index) {
                    graph[*from].insert(to);
                }
                // add self as parent
                if let Some(idx) = node.index {
                    parents.push((idx, d));
                } else {
                    must_exist.insert(node.dataset);
                }
                // add children to stack
                for node in node.children.values() {
                    stack.push((node.deref(), d + 1));
                }
            }
            for (to, child_dataset) in child_datasets.iter().enumerate() {
                if let Some(origin_dataset) = child_dataset.origin_dataset() {
                    if let Some(from) = root.get_str(&self.fs, origin_dataset).and_then(|t| t.index)
                    {
                        graph[from].insert(to);
                    } else {
                        info!(
                            "origin {origin_dataset} was excluded from sync, clone sync will fallback to full sync if {origin_dataset} does not exist in target"
                        );
                    };
                }
            }
            graph
        };
        // DFS graph for topological sort
        // We don't need to do any cycle detection because zfs can't create
        // clones/parent-child datesets in cycles
        let sorted = {
            let mut sorted = Vec::with_capacity(child_datasets.len());
            let mut seen = vec![false; child_datasets.len()];
            let mut stack = Vec::new();
            for idx in 0..child_datasets.len() {
                if !seen[idx] {
                    stack.push((idx, graph[idx].iter()));
                    seen[idx] = true;
                }
                while !stack.is_empty() {
                    let last = stack.last_mut().unwrap();
                    let current = last.0;
                    let remaining_children = &mut last.1;
                    if let Some(&next_child) = remaining_children.next() {
                        if !seen[next_child] {
                            stack.push((next_child, graph[next_child].iter()));
                            seen[next_child] = true;
                        }
                    } else {
                        sorted.push(current);
                        stack.pop();
                    }
                }
            }
            sorted.reverse();
            sorted
        };

        (sorted, must_exist)
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

#[cfg(test)]
mod test_topological {
    use super::*;

    fn appears_before_in(x: usize, y: usize, sorted: &[usize]) {
        let x_idx = sorted.iter().position(|&idx| idx == x).unwrap();
        let y_idx = sorted.iter().position(|&idx| idx == y).unwrap();
        assert!(x_idx < y_idx, "x:{x}:{x_idx} y:{y}:{y_idx}");
    }

    #[test]
    fn simple_linear() {
        let parent = Fs::new(None, "parent", Role::Target);
        let parent_copy = Fs::new(None, "parent", Role::Target);
        let child = Fs::new(None, "parent/child", Role::Target);
        let grand_child = Fs::new(None, "parent/child/grand_child", Role::Target);
        let unsorted = vec![grand_child, child, parent_copy];
        let (sorted, exists) = parent.topological_sort(&unsorted);
        assert!(exists.is_empty());
        assert_eq!(sorted, vec![2, 1, 0]);
    }

    #[test]
    fn simple_linear_2() {
        let parent = Fs::new(None, "parent", Role::Target);
        let parent_copy = Fs::new(None, "parent", Role::Target);
        let child = Fs::new(None, "parent/child", Role::Target);
        let grand_child = Fs::new(None, "parent/child/grand_child", Role::Target);
        let unsorted = vec![parent_copy, child, grand_child];
        let (sorted, exists) = parent.topological_sort(&unsorted);
        assert!(exists.is_empty());
        assert_eq!(sorted, vec![0, 1, 2]);
    }

    #[test]
    fn simple_cloned() {
        let parent = Fs::new(None, "parent", Role::Target);
        let parent_copy = Fs::new(None, "parent", Role::Target);
        let child = Fs::new(None, "parent/child", Role::Target);
        let mut cloned = Fs::new(None, "parent/cloned", Role::Target);
        cloned.origin = Some("parent/child@snap".to_string());
        let grand_child = Fs::new(None, "parent/child/grand_child", Role::Target);
        let unsorted = vec![parent_copy, cloned, child, grand_child];
        let (sorted, exists) = parent.topological_sort(&unsorted);
        assert!(exists.is_empty());
        assert!(sorted == vec![0, 2, 1, 3] || sorted == vec![0, 2, 3, 1]);
    }

    #[test]
    fn clone_in_sibling() {
        let parent = Fs::new(None, "parent", Role::Target);
        let parent_copy = Fs::new(None, "parent", Role::Target);
        let child_1 = Fs::new(None, "parent/child1", Role::Target);
        let mut clone = Fs::new(None, "parent/child1/clone", Role::Target);
        clone.origin = Some("parent/child2@snap".to_string());
        let child_2 = Fs::new(None, "parent/child2", Role::Target);
        let unsorted = vec![parent_copy, child_1, clone, child_2];
        let (sorted, exists) = parent.topological_sort(&unsorted);
        assert!(exists.is_empty());
        assert!(sorted == vec![0, 1, 3, 2] || sorted == vec![0, 3, 1, 2]);
    }

    #[test]
    fn example_from_syncoid_pr_572() {
        let test_pool = Fs::new(None, "testpool1", Role::Target);
        let mut a = Fs::new(None, "testpool1/A", Role::Target);
        a.origin = Some("testpool1/B@b".to_string());
        let a_d = Fs::new(None, "testpool1/A/D", Role::Target);
        let mut b = Fs::new(None, "testpool1/B", Role::Target);
        b.origin = Some("testpool1/C@a".to_string());
        let c = Fs::new(None, "testpool1/C", Role::Target);
        let unsorted = vec![test_pool, a, b, c, a_d];
        let (sorted, exists) = unsorted[0].topological_sort(&unsorted);
        assert!(exists.is_empty());
        for i in 1..unsorted.len() {
            appears_before_in(0, i, &sorted);
        }
        // A/D
        appears_before_in(1, 4, &sorted);
        // a.origin
        appears_before_in(2, 1, &sorted);
        // b.origin
        appears_before_in(3, 2, &sorted);
    }
}
