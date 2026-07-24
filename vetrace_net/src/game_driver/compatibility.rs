use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityManifest {
    pub protocol_version: u32,
    pub gameplay: BTreeMap<String, u64>,
}

impl CompatibilityManifest {
    pub fn new(protocol_version: u32) -> Self {
        Self {
            protocol_version,
            gameplay: BTreeMap::new(),
        }
    }

    pub fn with_gameplay_hash(mut self, name: impl Into<String>, hash: u64) -> Self {
        self.gameplay.insert(name.into(), hash);
        self
    }

    pub fn mismatch(&self, remote: &Self) -> Option<CompatibilityMismatch> {
        if self.protocol_version != remote.protocol_version {
            return Some(CompatibilityMismatch::Protocol {
                local: self.protocol_version,
                remote: remote.protocol_version,
            });
        }
        for (name, local) in &self.gameplay {
            match remote.gameplay.get(name) {
                Some(remote) if remote == local => {}
                Some(remote) => {
                    return Some(CompatibilityMismatch::Gameplay {
                        name: name.clone(),
                        local: *local,
                        remote: Some(*remote),
                    });
                }
                None => {
                    return Some(CompatibilityMismatch::Gameplay {
                        name: name.clone(),
                        local: *local,
                        remote: None,
                    });
                }
            }
        }
        for (name, remote) in &remote.gameplay {
            if !self.gameplay.contains_key(name) {
                return Some(CompatibilityMismatch::UnexpectedGameplay {
                    name: name.clone(),
                    remote: *remote,
                });
            }
        }
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompatibilityMismatch {
    Protocol {
        local: u32,
        remote: u32,
    },
    Gameplay {
        name: String,
        local: u64,
        remote: Option<u64>,
    },
    UnexpectedGameplay {
        name: String,
        remote: u64,
    },
}

impl fmt::Display for CompatibilityMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol { local, remote } => {
                write!(f, "protocol mismatch (local {local}, remote {remote})")
            }
            Self::Gameplay {
                name,
                local,
                remote: Some(remote),
            } => {
                write!(
                    f,
                    "{name} gameplay definitions differ (local {local:016x}, remote {remote:016x})"
                )
            }
            Self::Gameplay {
                name,
                local,
                remote: None,
            } => {
                write!(
                    f,
                    "remote is missing {name} gameplay definitions (local {local:016x})"
                )
            }
            Self::UnexpectedGameplay { name, remote } => {
                write!(
                    f,
                    "remote requires unknown {name} gameplay definitions ({remote:016x})"
                )
            }
        }
    }
}
