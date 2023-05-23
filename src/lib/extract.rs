// Copyright (c) 2020 Wind River Systems, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES
// OR CONDITIONS OF ANY KIND, either express or implied.

//! extract calls libarchive to extract the given archive

use std::fs::File;
use compress_tools::{uncompress_archive, Ownership, Result, Error};
use std::path::Path;

/// extract_archive uses libarchive to extract src to dst
pub fn extract_archive<S: AsRef<Path>, D: AsRef<Path>>(src: S, dst: D) -> Result<()> {
    let source = match File::open(src) {
        Ok(file) => file,
        Err(err) => return Err(Error::Io(err))
    };

    uncompress_archive(source, dst.as_ref(), Ownership::Ignore)
}

// list of known archive extensions
const VALID_EXTENSIONS: &'static [&'static str] = &["ar", "arj", "cpio", "dump", "jar", "7z", "zip", "pack", "pack2000", "tar", "bz2", "gz", "lzma", "snz", "xz", "z", "tgz", "rpm", "gem", "deb", "whl", "apk", "zst"];

/// is_extractable looks at the file extension, and possibly the context of files around it, to guess whether that file is an extractable file
pub fn is_extractable<P: AsRef<Path>>(path: P) -> u8 {
    match path.as_ref().extension() {
        None => 0,
        Some(ext) => {
            match ext.to_str() {
                None => 0, // no extension
                Some(s) => {
                    if s == "pack" { // If is a git pack file instead of pack200 file, it is not an archive
                        let mut idx_path = path.as_ref().to_path_buf();
                        let has_idx = match idx_path.set_extension("idx") {
                            true => idx_path.exists(),
                            false => false,
                        };

                        let in_objects_dir = match path.as_ref().parent() {
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
