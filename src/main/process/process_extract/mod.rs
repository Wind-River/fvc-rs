// Copyright (c) 2020 Wind River Systems, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES
// OR CONDITIONS OF ANY KIND, either express or implied.

//! Process given file paths and calculate file verification code
//! Internally it creates archive trees, which can be later traversed and fed to the FVC library, or can be used for debugging

use super::{ExtractPolicy, Processor};
use crate::FVC2Hasher;
use file_verification_code::FVCSha256Hasher;
mod dag;
use dag::{ArchiveGraph, EdgeResult};
use file_verification_code::extract;

use std::path::{Path, PathBuf};
use std::fs::metadata;
use log::*;
use walkdir::WalkDir;
use hex::ToHex;
use file_verification_code::archive_tree::{Directory, Archive, File, Collection};

pub struct ExtractionProcessor {
    extract_policy: ExtractPolicy,
}

impl Processor for ExtractionProcessor {
    fn new(extract_policy: ExtractPolicy) -> Self {
        Self { extract_policy: extract_policy }
    }

    fn calculate_fvc(self: &Self, hasher: &mut FVC2Hasher, files: &[PathBuf]) -> std::io::Result<()> {
        let mut collections: Vec<Collection> = Vec::new();
        for path in files {
            match self.calculate_fvc_of(&mut dag::ArchiveGraph::new(), None, path) {
                Ok(collection) => collections.push(collection),
                Err(err) => return Err(err)
            }
        }

        if log::log_enabled!(log::Level::Debug) {
            debug!("collections: {}", serde_json::to_string(&collections)?);
        }

        for collection in collections {
            ExtractionProcessor::hash_collection(hasher, collection);
        }
    
        Ok(())
    }
}

impl ExtractionProcessor {
    // extract_or_process_file looks at a path and applies the given extraction policy
    // On the extremes ExtractPolicy::None and ExtractPolicy::All will always or never process a path as an archive
    // ExtractPolicy::Extension will look at the file extension and extract it if it looks like an archive, otherwise it will process it as a file
    // The ArchiveGraph can skip looking at the path since it is already known to be an archive
    // In every case, if an archive fails to extract, due to an extraction-specific error, it is treated as a file
    // If a general IO error is encountered at any point, that is immediately returned
    fn extract_or_process_file<P: AsRef<Path>>(self: &Self, graph: &mut ArchiveGraph, current: Option<[u8; 32]>, file_path: P) -> std::io::Result<Collection> {
        match self.extract_policy {
            ExtractPolicy::None => match File::new(&file_path, None, None) { // nothing is to be extracted, immediately process as file
                Ok(file) => Ok(Collection::File(file)),
                Err(err) => Err(err)
            },
            ExtractPolicy::All | ExtractPolicy::Extension => {
                // calculate sha256 to check if file is an already known archive
                let sha256 = match get_sha256(&file_path) {
                    Ok(sha256) => sha256,
                    Err(err) => return Err(err)
                };
                let known_archive = ArchiveGraph::contains(graph, sha256);

                // if is an already known_archive, we might have a cycle
                match (known_archive, current) {
                    (true, Some(current)) => {
                        // check for cycle
                        match graph.add_edge(current, sha256) {
                            EdgeResult::Ok => (),
                            EdgeResult::CycleDetected => return Ok(Collection::Empty), // exit early to avoid cycle
                            EdgeResult::KeyMissing(key) => panic!("key missing for known archive? {}", key.encode_hex::<String>())
                        };

                        let mut archive = match Archive::new(&file_path, None, Some(sha256)) {
                            Ok(archive) => archive,
                            Err(err) => return Err(err)
                        };
                        // extract and process directory
                        match open_archive(&file_path) {
                            Ok(extracted_directory) => {
                                match self.calculate_fvc_of(graph, Some(sha256), extracted_directory.path()) {
                                    Ok(collection) => {
                                        match collection {
                                            Collection::File(file) => {
                                                archive.files.insert(file_path.as_ref().to_path_buf(), file);
                                            },
                                            Collection::Archive(archve) => {
                                                archive.archives.insert(file_path.as_ref().to_path_buf(), archve);
                                            },
                                            Collection::Directory(directory) => {
                                                archive.files = directory.files;
                                                archive.archives = directory.archives;
                                            },
                                            Collection::Empty => (),
                                        };
                                        match extracted_directory.close() { // clean up extraction
                                            Ok(()) => return Ok(Collection::Archive(archive)),
                                            Err(err) => return Err(err)
                                        };
                                    },
                                    Err(err) => return Err(err)
                                }
                            },
                            Err(err) => match err {
                                compress_tools::Error::Io(err) => return Err(err),
                                _ => panic!("archive error for known archive: {}", err)
                            }
                        }

                    },
                    (true, None) => {
                        // no cycle possible
                        let mut archive = match Archive::new(&file_path, None, Some(sha256)) {
                            Ok(archive) => archive,
                            Err(err) => return Err(err)
                        };
                        // extract and process directory
                        match open_archive(&file_path) {
                            Ok(extracted_directory) => {
                                match self.calculate_fvc_of(graph, Some(sha256), extracted_directory.path()) {
                                    Ok(collection) => {
                                        match collection {
                                            Collection::File(file) => {
                                                archive.files.insert(file_path.as_ref().to_path_buf(), file);
                                            },
                                            Collection::Archive(archve) => {
                                                archive.archives.insert(file_path.as_ref().to_owned(), archve);
                                            }
                                            Collection::Directory(directory) => {
                                                archive.files = directory.files;
                                                archive.archives = directory.archives;
                                            },
                                            Collection::Empty => ()
                                        };
                                        match extracted_directory.close() { // clean up extraction
                                            Ok(()) => return Ok(Collection::Archive(archive)),
                                            Err(err) => return Err(err)
                                        };
                                    },
                                    Err(err) => return Err(err)
                                }
                            },
                            Err(err) => match err {
                                compress_tools::Error::Io(err) => return Err(err),
                                _ => panic!("archive error for known archive: {}", err)
                            }
                        }
                    }
                    (false, _) => ()
                };

                // unknown if archive or file
                // return early if archive was extracted and processed, otherwise fall to file process below
                match (self.extract_policy, extract::is_extractable(&file_path)) {
                    (ExtractPolicy::Extension, 0) => (),
                    (_, 100) => {
                        let mut archive = match Archive::new(&file_path, None, Some(sha256)) {
                            Ok(archive) => archive,
                            Err(err) => return Err(err)
                        };
                        match open_archive(&file_path) {
                            Err(err) => match err {
                                compress_tools::Error::Io(err) => return Err(err),
                                _ => debug!("error extracting 100 confidence archive: {}", file_path.as_ref().display())
                            },
                            Ok(extracted_directory) => {
                                graph.insert(sha256);
                                match self.calculate_fvc_of(graph, Some(sha256), extracted_directory.path()) {
                                    Ok(collection) => {
                                        match collection {
                                            Collection::File(file) => {
                                                archive.files.insert(file_path.as_ref().to_path_buf(), file);
                                            },
                                            Collection::Archive(archve) => {
                                                archive.archives.insert(file_path.as_ref().to_owned(), archve);
                                            },
                                            Collection::Directory(directory) => {
                                                archive.files = directory.files;
                                                archive.archives = directory.archives;
                                            },
                                            Collection::Empty => ()
                                        };
                                        match extracted_directory.close() { // clean up extraction
                                            Ok(()) => return Ok(Collection::Archive(archive)),
                                            Err(err) => return Err(err)
                                        };
                                    },
                                    Err(err) => return Err(err)
                                }
                            }
                        }
                    },
                    (_, _confidence) => {
                        // for now, we try to extract anything over 0, so this arm is the same as ExtractPolicy::All
                        match open_archive(&file_path) {
                            Err(err) => match err {
                                compress_tools::Error::Io(err) => return Err(err),
                                _ => ()
                            },
                            Ok(extracted_directory) => {
                                graph.insert(sha256);
                                let mut archive = match Archive::new(&file_path, None, Some(sha256)) {
                                    Ok(archive) => archive,
                                    Err(err) => return Err(err)
                                };
                                match self.calculate_fvc_of(graph, Some(sha256), extracted_directory.path()) {
                                    Ok(collection) => {
                                        match collection {
                                            Collection::File(file) => {
                                                archive.files.insert(file_path.as_ref().to_path_buf(), file);
                                            },
                                            Collection::Archive(archve) => {
                                                archive.archives.insert(file_path.as_ref().to_path_buf(), archve);
                                            },
                                            Collection::Directory(directory) => {
                                                archive.files = directory.files;
                                                archive.archives = directory.archives;
                                            },
                                            Collection::Empty => ()
                                        };
                                        match extracted_directory.close() { // clean up extraction
                                            Ok(()) => return Ok(Collection::Archive(archive)),
                                            Err(err) => return Err(err)
                                        };
                                    },
                                    Err(err) => return Err(err)
                                }
                            }
                        }
                    }
                }

                // was not able to, or decided not to, process as an archive
                match File::new(&file_path, None, None) {
                    Ok(file) => Ok(Collection::File(file)),
                    Err(err) => Err(err)
                }
            }
        }
    }

    // calculate_fvc_of acts like calculate_fvc, buts adds the ArchiveGraph and current archive to protect against quines
    // the archive graph is a directed acyclic graph, and if a cycle is ever detected, that edge is not added, and thus that archive is not processed futher
    fn calculate_fvc_of(self: &Self, graph: &mut ArchiveGraph, current: Option<[u8; 32]>, filepath: &Path) -> std::io::Result<Collection> {
        let stat = match metadata(filepath) {
            Ok(metadata) => metadata,
            Err(err) => {
                return Err(err);
            }
        };

        if stat.is_file() {
            return self.extract_or_process_file(graph, current, filepath);            
        } else if stat.is_dir() {
            info!("Adding directory \"{}\"", filepath.display());
            let mut directory = Directory::new(filepath);

            for entry in WalkDir::new(filepath) {
                let dir_entry = match entry {
                    Ok(dir_entry) => dir_entry,
                    Err(err) => {
                        log::error!("error walking dir: {}", err);
                        return Err(err.into()); // walkdir::Error is a light wrapper around std::io::Error
                    }
                };
                trace!("at entry {}", dir_entry.path().display());

                // only process files
                if dir_entry.file_type().is_file() {
                    trace!("trying file {}", dir_entry.path().display());
                    match self.extract_or_process_file(graph, current, dir_entry.path()) {
                        Ok(collection) => match collection {
                            Collection::Directory(_) => panic!("WalkDir should be ignoring directories and returning files directly"),
                            Collection::File(file) => {
                                directory.files.insert(dir_entry.path().to_owned(), file);
                            },
                            Collection::Archive(archive) => {
                                directory.archives.insert(dir_entry.path().to_owned(), archive);
                            },
                            Collection::Empty => ()
                        },
                        Err(err) => {
                            log::error!("error processing file {}", dir_entry.path().display());
                            return Err(err);
                        }
                    }
                }
            }

            return Ok(Collection::Directory(directory));
        } else {
            info!("Skipping irregular file {}", filepath.display());
        }
    
        Ok(Collection::Empty)
    }

    // hash_collection process the given collection and feeds its files to the FVC2Hasher
    fn hash_collection(hasher: &mut FVC2Hasher, collection: Collection) {
        match collection {
            Collection::Empty => (),
            Collection::File(file) => hasher.read_sha256(file.sha256),
            Collection::Archive(archive) => {
                for (_path, file) in archive.files {
                    hasher.read_sha256(file.sha256);
                }
                for (_path, archive) in archive.archives {
                    ExtractionProcessor::hash_collection(hasher, Collection::Archive(archive))
                }
            },
            Collection::Directory(directory) => {
                for (_path, file) in directory.files {
                    hasher.read_sha256(file.sha256);
                }
                for (_path, archive) in directory.archives {
                    ExtractionProcessor::hash_collection(hasher, Collection::Archive(archive))
                }                
            },
        }
    }
}

// get_sha256 calculates and returns an array of bytes represeting the sha256 of the given file
fn get_sha256<P: AsRef<Path>>(path: P) -> std::io::Result<[u8; 32]> {
    use sha2::{Sha256, Digest};
    use std::io::Read;

    let mut hasher = Sha256::new();
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(err) => return Err(err)
    };
    let mut buf = Vec::new();
    let sha256: [u8; 32] = match file.read_to_end(&mut buf) {
        Ok(_size) => {
            hasher.update(buf);
            hasher.finalize().into()
        },
        Err(err) => return Err(err)
    };

    Ok(sha256)
}

// open archive creates a temporary directory and extracts the given archive to it
// in the case of an extraction error, the temporary directory is cleaned-up here, otherwise it needs to be cleaned up by the receiever
fn open_archive<P: AsRef<Path>>(archive_path: P) -> compress_tools::Result<tempdir::TempDir> {
    let tmp_prefix = match archive_path.as_ref().file_name() {
        Some(file_name) => format!("fvc_extracted_archive.{:?}", file_name),
        None => format!("fvc_extracted_archive.{:?}", archive_path.as_ref())
    };
    let tmp = match tempdir::TempDir::new(&tmp_prefix) {
        Ok(tmp) => tmp,
        Err(err) => return Err(compress_tools::Error::Io(err))
    };

    match extract::extract_archive(&archive_path, tmp.as_ref()) {
        Ok(()) => {
            info!("extracted archive {}", archive_path.as_ref().display());
            Ok(tmp)
        },
        Err(err) => {
            match tmp.close() { // explicitly clean-up tmp directory to be able to log errors
                Ok(()) => (),
                Err(err) => debug!("error closing tmp directory: {}", err)
            };
            Err(err)
        }
    }
}