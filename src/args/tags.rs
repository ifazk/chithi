use log::error;
use std::{collections::HashSet, io};

pub enum TagFilter<'args> {
    Untagged,
    Tagged {
        include: Vec<&'args str>,
        exclude: Vec<&'args str>,
    },
}

impl<'args> TagFilter<'args> {
    pub fn parse(value: &'args str) -> io::Result<Self> {
        let value = value.trim();
        match value {
            "none" => Ok(Self::Untagged),
            tags => {
                let mut include = Vec::new();
                let mut exclude = Vec::new();
                for tag in tags.split(',') {
                    if tag.starts_with(['!', '/']) {
                        let tag = &tag[1..];
                        Self::check_tag(tag)?;
                        exclude.push(tag);
                    } else {
                        Self::check_tag(tag)?;
                        include.push(tag);
                    }
                }
                Ok(Self::Tagged { include, exclude })
            }
        }
    }
    pub fn matches(&self, item_tags: &HashSet<String>) -> bool {
        match self {
            TagFilter::Untagged => item_tags.is_empty(),
            TagFilter::Tagged { include, exclude } => {
                include.iter().all(|&t| item_tags.contains(t))
                    && exclude.iter().all(|&t| !item_tags.contains(t))
            }
        }
    }
    fn check_tag(tag: &str) -> io::Result<()> {
        if tag.is_empty() {
            error!("empty string should not be used as a tag");
            return Err(io::Error::other("found empty string tag in project"));
        }
        // special error message for none
        if tag == "none" {
            error!(
                "'none' matches untagged items and it is not meaningful to combine it with other tags"
            )
        }
        const RESERVED: [&str; 9] = ["any", "all", "and", "or", "not", "|", "||", "&", "&&"];
        if RESERVED.contains(&tag) {
            error!(
                "any, all, and, or, not, '||', '|', '&', '&&' are currently reserved and do not match against anything"
            );
            return Err(io::Error::other(format!(
                "use a reserved word as a search tag '{}'",
                tag
            )));
        }
        if tag
            .chars()
            .any(|c| c == '(' || c == ')' || c == '"' || c == '\'' || c.is_whitespace())
        {
            error!(
                "tags cannot contain parentheses, quotes, or whitespace characters, invalid tag \"{}\"",
                tag.escape_default()
            );
            return Err(io::Error::other(format!(
                "invalid tag \"{}\"",
                tag.escape_default()
            )));
        }
        Ok(())
    }
}
