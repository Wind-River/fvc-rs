// Copyright (c) 2020 Wind River Systems, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES
// OR CONDITIONS OF ANY KIND, either express or implied.

//! There are a type of zip bombs that are quines, archives that extract into exact copies of themselves.
//! To project against this a Directed Acyclic Graph can be maintained to detect when an infinite loop is about to be created. 

use std::collections;
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

    // add_edge adds the given edge to the graph iff a cycle won't be created by doing so
    pub fn add_edge(self: &mut Self, from: [u8; 32], to: [u8; 32]) -> EdgeResult {
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

    // archive_can_find is used before adding an edge to see if the destination can already reach the start before adding the edge
    // if it can reach, the new edge would be creating a cycle
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

        for sub_archive in &start.sub_archives {
            match self.archives.get(sub_archive) {
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