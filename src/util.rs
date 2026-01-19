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

use std::fmt::Display;

pub struct ReadableBytes(u64);

impl From<u64> for ReadableBytes {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl Display for ReadableBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;

        if self.0 == 0 {
            write!(f, "UNKNOWN")?;
        } else if self.0 >= GB {
            let gb = self.0 as f64 / GB as f64;
            write!(f, "{gb:.1} GiB")?;
        } else if self.0 >= MB {
            let mb = self.0 as f64 / MB as f64;
            write!(f, "{mb:.1} MiB")?;
        } else {
            let kb = self.0 / KB;
            write!(f, "{} KiB", kb)?;
        }
        Ok(())
    }
}

pub struct SpaceSeparatedStrings<T>(pub T);

impl<U: AsRef<[String]>> Display for SpaceSeparatedStrings<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.0.as_ref().iter();
        if let Some(t) = iter.next() {
            write!(f, "{t}")?;
        };
        for t in iter {
            write!(f, " {t}")?;
        }
        Ok(())
    }
}

pub struct SpaceSeparatedStrs<T>(pub T);

impl<'a, U: AsRef<[&'a str]>> Display for SpaceSeparatedStrs<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.0.as_ref().iter();
        if let Some(t) = iter.next() {
            write!(f, "{t}")?;
        };
        for t in iter {
            write!(f, " {t}")?;
        }
        Ok(())
    }
}

pub struct SpaceSeparatedCowStrs<T>(pub T);

impl<'a, U: AsRef<[std::borrow::Cow<'a, str>]>> Display for SpaceSeparatedCowStrs<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.0.as_ref().iter();
        if let Some(t) = iter.next() {
            write!(f, "{t}")?;
        };
        for t in iter {
            write!(f, " {t}")?;
        }
        Ok(())
    }
}

pub struct OptDisplay<T>(pub T);

impl<T: Display> Display for OptDisplay<Option<&T>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(t) = self.0 {
            write!(f, "{t}")?;
        }
        Ok(())
    }
}
