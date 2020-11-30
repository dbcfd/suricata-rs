use crate::intel::{CachedRule, IdsKey, IntelCache, Tracer};
use crate::Error;

use lazy_static::lazy_static;
use log::*;
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct Rule {
    key: IdsKey,
    rule: String,
}

impl AsRef<[u8]> for Rule {
    fn as_ref(&self) -> &[u8] {
        self.rule.as_bytes()
    }
}

lazy_static! {
    pub static ref SID_REGEX: Regex =
        Regex::from_str(r#".+sid\s*:\s*(\d+);.+"#).expect("Bad regex");
    pub static ref GID_REGEX: Regex =
        Regex::from_str(r#".+gid\s*:\s*(\d+);.+"#).expect("Bad regex");
}

fn parse_rule(line: &str) -> Result<Rule, Error> {
    let caps = SID_REGEX.captures(line).ok_or(Error::Custom {
        msg: format!("No sid: {}", line),
    })?;
    let sid = &caps[1];
    let sid = u64::from_str(sid).map_err(Error::ParseInt)?;
    let gid = if let Some(gid) = GID_REGEX.captures(line) {
        u64::from_str(&gid[1]).map_err(Error::from)?
    } else {
        1
    };
    Ok(Rule {
        key: IdsKey { gid: gid, sid: sid },
        rule: line.to_owned(),
    })
}

pub struct Rules {
    inner: Vec<Rule>,
}

impl Rules {
    pub fn from_path<T: AsRef<Path>>(path: T) -> Result<Self, Error> {
        let f = File::open(path.as_ref()).map_err(Error::from)?;
        let lines: Result<Vec<_>, Error> = BufReader::new(f)
            .lines()
            .map(|r| r.map_err(Error::from))
            .collect();
        let lines = lines?;
        let rules: Vec<_> = lines
            .into_iter()
            .flat_map(|l| {
                let l = l.trim_start();
                if l.is_empty() || l.starts_with("#") {
                    None
                } else {
                    match parse_rule(l) {
                        Ok(r) => Some(r),
                        Err(e) => {
                            warn!("Failed to parse rule '{}': {:?}", l, e);
                            None
                        }
                    }
                }
            })
            .collect();
        Ok(Self { inner: rules })
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl Into<IntelCache<Rule>> for Rules {
    fn into(self) -> IntelCache<Rule> {
        let mut map: HashMap<IdsKey, CachedRule<Rule>> = self
            .inner
            .into_iter()
            .map(|r| (r.key.clone(), CachedRule::Ids(r)))
            .collect();
        map.insert(Tracer::key(), Tracer::rule::<Rule>());
        IntelCache { inner: map }
    }
}
