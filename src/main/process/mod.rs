// Copyright (c) 2020 Wind River Systems, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES
// OR CONDITIONS OF ANY KIND, either express or implied.

mod extract;
use extract::extract_archive;
use crate::{FVCHasher, FVC2Hasher};

use walkdir::WalkDir;
use std::fs::{metadata, File};
use std::path::{Path, PathBuf};
use log::{warn, info, trace};
use clap::ValueEnum;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum ExtractPolicy {
    /// Only try to extract files with extensions that look like archives
    Extension,
    /// Try to extract every file
    All,
    /// Don't extract, treat archives as binary files
    None    
}

/// calculate_fvc iterates over the given files and adds them to the FVCHasher, or extracts and/or walk given archives/directories and does the same for their files.
/// The actual fvc at the end can be obtained from the given hasher.
// TODO add protection against archive quines.
pub fn calculate_fvc(hasher: &mut FVC2Hasher, extract_policy: ExtractPolicy, files: &[PathBuf]) -> std::io::Result<()> {
    for path in files {
        let stat = match metadata(path) {
            Ok(metadata) => metadata,
            Err(err) => {
                return Err(err);
            }
        };

        if stat.is_file() {
            match extract_or_process_file(hasher, extract_policy, path) {
                Ok(()) => (),
                Err(err) => {
                    return Err(err);
                }
            }
        } else if stat.is_dir() {
            info!("Adding directory \"{}\"", path.display());

            for entry in WalkDir::new(path) {
                let entry = match entry {
                    Ok(dir_entry) => dir_entry,
                    Err(err) => {
                        return Err(err.into()); // walkdir::Error is a light wrapper around std::io::Error
                    }
                };

                if entry.file_type().is_file() {
                    match extract_or_process_file(hasher, extract_policy, entry.path()) {
                        Ok(()) => (),
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }
            }
        } else {
            info!("Skipping irregular file {}", path.display());
        }
    }

    return Ok(())
}

fn extract_or_process_file(hasher: &mut FVC2Hasher, extract_policy: ExtractPolicy, file_path: &Path) -> std::io::Result<()> {
    match (extract_policy, is_extractable(file_path)) {
        (ExtractPolicy::Extension, 0) | (ExtractPolicy::None, _) => {
            // Don't extract any files, or file does not look extractable
            match process_file(hasher, file_path) {
                Ok(()) => Ok(()),
                Err(err) => Err(err)
            }
        },
        (ExtractPolicy::Extension, _confidence) => {
            // Archive looks extractable
            match process_archive(hasher, extract_policy, file_path) {
                Ok(()) => Ok(()),
                Err(err) => Err(err)
            }
        },
        (ExtractPolicy::All, _) => {
            // Try processing every archive
            match process_archive(hasher, extract_policy, file_path) {
                Ok(()) => Ok(()),
                Err(err) => Err(err)
            }
        },
    }
}

fn process_file(hasher: &mut FVC2Hasher, file_path: &Path) -> std::io::Result<()> {
    trace!("Adding file \"{}\"", file_path.display());
    // open file
    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };
    // pass open file to hasher
    match hasher.read(file) {
        Ok(_size) => Ok(()),
        Err(err) => {
            return Err(err);
        }
    }
}

fn process_archive(hasher: &mut FVC2Hasher, extract_policy: ExtractPolicy,archive_path: &Path) -> std::io::Result<()> {
    let tmp_prefix = match archive_path.file_name() {
        Some(file_name) => format!("fvc_extracted_archive.{:?}", file_name),
        None => format!("fvc_extracted_archive.{:?}", archive_path)
    };
    let tmp = match tempdir::TempDir::new(&tmp_prefix) {
        Ok(tmp) => tmp,
        Err(err) => return Err(err)
    };
    match extract_archive(&archive_path, tmp.path()) {
        Ok(()) => {
            info!("extracted archive {}", archive_path.display());
            match calculate_fvc(hasher, extract_policy, &[tmp.path().to_owned()]) {
                Ok(()) => (),
                Err(err) => {
                    return Err(err)
                }
            };
        },
        Err(err) => {
            match extract_policy {
                // If we're trying to extract every file, log as trace instead of warn
                ExtractPolicy::All => trace!("error extracting {}, treating as file: {}", archive_path.display(), err),
                // Warn when an extraction that we reasonably expected to succeed, instead fails and is treated as a file
                _ => warn!("error extracting {}, treating as file: {}", archive_path.display(), err)
            }
            return process_file(hasher, archive_path);
        }
    };
    // if we rely on tmp destructor to clean, errors are ignored
    tmp.close().expect("closing extracted tempdir");
    Ok(())
}

// list of known archive extensions
const VALID_EXTENSIONS: &'static [&'static str] = &["ar", "arj", "cpio", "dump", "jar", "7z", "zip", "pack", "pack2000", "tar", "bz2", "gz", "lzma", "snz", "xz", "z", "tgz", "rpm", "gem", "deb", "whl", "apk", "zst"];

/// is_extractable looks at the file extension, and possibly the context of files around it, to guess whether that file is an extractable file
pub fn is_extractable(path: &Path) -> u8 {
    match path.extension() {
        None => 0,
        Some(ext) => {
            match ext.to_str() {
                None => 0, // no extension
                Some(s) => {
                    if s == "pack" {
                        // TODOC, I think I was checking here whether it was a git pack file or a pack2000
                        let mut idx_path = path.to_path_buf();
                        let has_idx = match idx_path.set_extension("idx") {
                            true => idx_path.exists(),
                            false => false,
                        };

                        let in_objects_dir = match path.parent() {
                            None => false,
                            Some(parent) => {
                                match parent.to_str() {
                                    Some("objects") => true,
                                    _ => false
                                }
                            }
                        };

                        if has_idx && in_objects_dir {
                            return 0
                        } else if has_idx || in_objects_dir {
                            return 50
                        } else {
                            return 100
                        }
                    } else {
                        for valid in VALID_EXTENSIONS {
                            if s == *valid {
                                return 100
                            }
                        }
                    }

                    0
                }
            }
        }
    }
}