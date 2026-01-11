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

use std::{fmt::Display, mem};

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
                            param: None,
                        });
                    }
                }
            } else {
                let option = last_option
                    .expect("parsing_options should only be false when last_option contains value");
                options.push(OptionsLine {
                    option,
                    param: Some(s.to_string()),
                });
                parsing_options = true;
                last_option = None;
            }
        }
        if last_option.is_some() {
            return Err("did not find value after o, x, or X option");
        }
        Ok(Self { options })
    }

    pub fn filter_allowed(&self, allowed: &'static [char]) -> Vec<String> {
        let filtered = self.options.iter().filter(|o| allowed.contains(&o.option));
        let mut dashed = String::from("-");
        let mut res = Vec::new();
        for opt in filtered {
            dashed.push(opt.option);
            if let Some(param) = opt.param.clone() {
                let mut swap_dashed = String::from("-");
                mem::swap(&mut dashed, &mut swap_dashed);
                res.push(swap_dashed);
                res.push(param);
            }
        }
        if dashed.len() > 1 {
            res.push(dashed)
        };
        res
    }
}

impl Display for Opts<Vec<OptionsLine<String>>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dash_printed = false;
        let mut after_param = false;
        for opt in &self.options {
            if after_param {
                write!(f, " ")?;
                after_param = false;
            }
            if !dash_printed {
                write!(f, "-")?;
                dash_printed = true;
            }
            write!(f, "{}", opt.option)?;
            if let Some(param) = opt.param.as_ref() {
                write!(f, " {}", param)?;
                dash_printed = false;
                after_param = true;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct OptionsLine<T> {
    pub option: char,
    pub param: Option<T>,
}
