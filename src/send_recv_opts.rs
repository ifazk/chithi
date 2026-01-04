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

// TODO decide if we want to offer a different cli for send/recv options

use std::fmt::Display;

#[derive(Debug, Clone, Default)]
pub struct Opts<T> {
    pub options: T,
}

impl Opts<Vec<OptionsLine<String>>> {
    pub fn try_from_str(value: &str) -> Result<Self, &'static str> {
        // 2 state dfa, using bool
        let mut parsing_options = true;
        let mut last_option = None;
        let mut options = Vec::new();
        for s in value.split(' ') {
            if s.is_empty() {
                continue;
            }
            if parsing_options {
                for c in s.chars() {
                    if last_option.is_some() {
                        return Err(
                            "found another single letter options after o, x, or X instead of the option value",
                        );
                    }
                    if ['o', 'x', 'X'].contains(&c) {
                        last_option = Some(c);
                        parsing_options = false
                    } else {
                        options.push(OptionsLine {
                            option: c,
                            line: format!("-{c}"),
                        });
                    }
                }
            } else {
                let option = last_option
                    .expect("parsing_options should only be false when last_option contains value");
                let line = format!("-{option} {s}");
                options.push(OptionsLine { option, line });
                parsing_options = true;
                last_option = None;
            }
        }
        if last_option.is_some() {
            return Err("did not find value after o, x, or X option");
        }
        Ok(Self { options })
    }

    pub fn filter_allowed(&self, allowed: &'static [char]) -> Vec<&str> {
        self.options
            .iter()
            .filter_map(|o| {
                if allowed.contains(&o.option) {
                    Some(o.line.as_str())
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Display for Opts<Vec<OptionsLine<String>>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut after_opt = false;
        for opt in &self.options {
            if after_opt {
                write!(f, " ")?;
                after_opt = false;
            }
            let line = &opt.line.as_str()[1..];
            write!(f, "{}", line)?;
            if ['o', 'x', 'X'].contains(&opt.option) {
                after_opt = true;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct OptionsLine<T> {
    pub option: char,
    pub line: T,
}
