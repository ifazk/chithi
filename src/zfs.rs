//  Chithi: OpenZFS replication tools
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

use std::collections::HashMap;

pub fn is_component_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':' || c == '.'
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Creation {
    pub creation: u64,
    idx: usize,
}

impl Creation {
    pub fn new(creation: &str, idx: usize) -> Option<Self> {
        let creation = creation.parse::<u64>().ok()?;
        Some(Self { creation, idx })
    }
    pub fn fake_new(creation: u64, idx: usize) -> Self {
        Self { creation, idx }
    }
}

impl std::fmt::Display for Creation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:010}{:03}", self.creation, self.idx)
    }
}

/// A type that represents a snapshot or bookmark.
/// We abstract over the type, but in reality T is either String or &str
pub struct Snapshot<T> {
    pub name: T,
    pub guid: T,
    pub creation: Creation,
}

impl<T> Snapshot<T> {
    pub fn new(name: T, guid: T, creation: Creation) -> Self {
        Self {
            name,
            guid,
            creation,
        }
    }
}

impl Snapshot<String> {
    pub fn fake_newest(name: String) -> Self {
        const FAKE_NEW_SYNC_GUID: &str = "9999999999999999999";
        const FAKE_NEW_SYNC_CREATION: u64 = u64::MAX;
        Self {
            name,
            guid: FAKE_NEW_SYNC_GUID.to_string(),
            creation: Creation::fake_new(FAKE_NEW_SYNC_CREATION, 0),
        }
    }
    pub fn list_to_map(list: &[Self]) -> HashMap<&str, SnapshotInfo<&str>> {
        list.iter()
            .map(|snapshot| (snapshot.name.as_str(), snapshot.into()))
            .collect::<HashMap<_, _>>()
    }
}

impl<'a> From<&'a Snapshot<String>> for Snapshot<&'a str> {
    fn from(value: &'a Snapshot<String>) -> Self {
        Self {
            name: &value.name,
            guid: &value.guid,
            creation: value.creation,
        }
    }
}

impl<'a, 'b> From<&'a Snapshot<&'b str>> for Snapshot<String> {
    fn from(value: &'a Snapshot<&'b str>) -> Self {
        Self {
            name: value.name.to_string(),
            guid: value.guid.to_string(),
            creation: value.creation,
        }
    }
}

pub enum IntermediateSource<'a> {
    Snapshot(Snapshot<&'a str>),
    Bookmark(Snapshot<&'a str>, &'a str),
}

impl<'a> IntermediateSource<'a> {
    pub fn source(&self) -> String {
        match self {
            IntermediateSource::Snapshot(snapshot) => format!("@{}", snapshot.name),
            IntermediateSource::Bookmark(snapshot, _target) => format!("#{}", snapshot.name),
        }
    }
    pub fn kind(&self) -> &'static str {
        match self {
            IntermediateSource::Snapshot(_) => "snapshot",
            IntermediateSource::Bookmark(_, _) => "bookmark",
        }
    }
    pub fn creation(&self) -> u64 {
        match self {
            IntermediateSource::Snapshot(snapshot) => snapshot.creation.creation,
            IntermediateSource::Bookmark(snapshot, _) => snapshot.creation.creation,
        }
    }
}

/// SnapshotInfo is mostly useful for hashmaps. We abstract over the type, but
/// in reality T is either String or &str
pub struct SnapshotInfo<T> {
    pub guid: T,
    pub creation: Creation,
}

impl<T: Clone> From<Snapshot<T>> for SnapshotInfo<T> {
    fn from(value: Snapshot<T>) -> Self {
        Self {
            guid: value.guid.clone(),
            creation: value.creation,
        }
    }
}

impl<T: Clone> From<&Snapshot<T>> for SnapshotInfo<T> {
    fn from(value: &Snapshot<T>) -> Self {
        Self {
            guid: value.guid.clone(),
            creation: value.creation,
        }
    }
}

impl<'a> From<&'a Snapshot<String>> for SnapshotInfo<&'a str> {
    fn from(value: &'a Snapshot<String>) -> Self {
        Self {
            guid: value.guid.as_str(),
            creation: value.creation,
        }
    }
}
