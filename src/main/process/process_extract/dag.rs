use std::collections;
use hex::ToHex;
pub struct ArchiveGraph {
    archives: collections::HashMap<[u8; 32], Archive>
}

pub enum EdgeResult {
    Ok,
    CycleDetected,
    KeyMissing([u8; 32])
}

impl ArchiveGraph {
    pub fn new() -> Self {
        ArchiveGraph {
            archives: collections::HashMap::new()
        }
    }

    pub fn insert(self: &mut Self, sha256: [u8; 32]) {
        self.archives.insert(sha256, Archive::new(sha256));
    }

    pub fn add_edge(self: &mut Self, from: [u8; 32], to: [u8; 32]) -> EdgeResult {
        log::trace!("trying to add_edge {} -> {}", from.encode_hex::<String>(), to.encode_hex::<String>());
        if self.archive_can_find(to, from) {
            return EdgeResult::CycleDetected;
        }

        match self.archives.get_mut(&from) {
            None => {
                return EdgeResult::KeyMissing(from);
            },
            Some(from) => {
                from.sub_archives.push(to);
            }
        }

        EdgeResult::Ok
    }

    pub fn contains(haystack: &Self, needle: [u8; 32]) -> bool {
        haystack.archives.contains_key(&needle)
    }

    pub fn archive_can_find(self: &Self, start: [u8; 32], destination: [u8; 32]) -> bool {
        if start == destination {
            return true;
        }

        let start = match self.archives.get(&start) {
            None => {
                return false;
            },
            Some(start) => start
        };
        let destination = match self.archives.get(&destination) {
            None => {
                return false;
            },
            Some(destination) => destination
        };

        if start.sub_archives.contains(&destination.sha256) {
            return true;
        }

        for a in &start.sub_archives {
            match self.archives.get(a) {
                None => (),
                Some(sub_archive) => {
                    if self.archive_can_find(sub_archive.sha256, destination.sha256) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

struct Archive {
    sha256: [u8; 32],
    sub_archives: std::vec::Vec<[u8; 32]>
}

impl Archive {
    fn new(sha256: [u8; 32]) -> Archive {
        Archive {
            sha256: sha256,
            sub_archives: std::vec::Vec::new()
        }
    }
}