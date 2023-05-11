// internal imports
mod extract;
use extract::{extract_archive, ExtractResult};
use crate::{CLI, FVCHasher, FVC2Hasher};

// external imports
use walkdir::WalkDir;
use std::fs::{metadata, File};
use std::path::{Path, PathBuf};

/// calculate_fvc iterates over the given files and adds them to the FVCHasher, or extracts and/or walk given archives/directories and does the same for their files.
/// The actual fvc at the end can be obtained from the given hasher.
// TODO add protection against archive quines.
pub fn calculate_fvc(cli: &CLI, hasher: &mut FVC2Hasher, files: &[PathBuf]) -> std::io::Result<()> {
    for path in files {
        let stat = match metadata(path) {
            Ok(metadata) => metadata,
            Err(err) => {
                return Err(err);
            }
        };

        if stat.is_file() {
            if is_extractable(path) > 0. {
                let tmp = match tempdir::TempDir::new("") {
                    Ok(tmp) => tmp,
                    Err(err) => return Err(err)
                };
                match extract_archive(&path, tmp.path()) {
                    ExtractResult::Ok => {
                        if cli.log_verbose() {
                            eprintln!("extracted archive {}", path.display());
                        }
                        match calculate_fvc(&cli, hasher, &[tmp.path().to_owned()]) {
                            Ok(()) => (),
                            Err(err) => {
                                eprintln!("error processing extract archive {:#?}", err);
                                return Err(err)
                            }
                        };
                    },
                    ExtractResult::ArchiveError(err) => {
                        eprintln!("error reading archive {:#?}", err);
                    }
                };
                // if we rely on tmp destructor to clean, errors are ignored
                tmp.close().expect("closing extracted tempdir");
            } else {
                if cli.log_verbose() {
                    eprintln!("Adding file \"{}\"", path.display());
                }
                // open file
                let file = match File::open(path) {
                    Ok(file) => file,
                    Err(err) => {
                        eprintln!("{}", err);
                        std::process::exit(1);
                    }
                };
                // pass open file to hasher
                match hasher.read(file) {
                    Ok(_size) => (),
                    Err(err) => {
                        return Err(err);
                    }
                }
            }
        } else if stat.is_dir() {
            if cli.log_verbose() {
                eprintln!("Adding directory \"{}\"", path.display());
            }

            for entry in WalkDir::new(path) {
                let entry = match entry {
                    Ok(dir_entry) => dir_entry,
                    Err(err) => {
                        return Err(err.into()); // walkdir::Error is a light wrapper around std::io::Error
                    }
                };

                if entry.file_type().is_file() {
                    if cli.log_debug() {
                        eprintln!("Adding file \"{}\" from directory \"{}\"", entry.path().display(), path.display());
                    }
                    // open file
                    let file = match File::open(entry.path()) {
                        Ok(file) => file,
                        Err(err) => {
                            return Err(err)
                        }
                    };
                    // pass file to hasher
                    match hasher.read(file) {
                        Ok(_size) => (),
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }
            }
        } else {
            if cli.log_verbose() {
                eprintln!("Skipping irregular file {}", path.display());
            }
        }
    }

    return Ok(())
}

// list of known archive extensions
const VALID_EXTENSIONS: &'static [&'static str] = &["ar", "arj", "cpio", "dump", "jar", "7z", "zip", "pack", "pack2000", "tar", "bz2", "gz", "lzma", "snz", "xz", "z", "tgz", "rpm", "gem", "deb", "whl", "apk", "zst"];

/// is_extractable looks at the file extension, and possibly the context of files around it, to guess whether that file is an extractable file
pub fn is_extractable(path: &Path) -> f32 {
    match path.extension() {
        None => 0.,
        Some(ext) => {
            match ext.to_str() {
                None => 0., // no extension
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
                            return 0.
                        } else if has_idx || in_objects_dir {
                            return 0.5
                        } else {
                            return 1.
                        }
                    } else {
                        for valid in VALID_EXTENSIONS {
                            if s == *valid {
                                return 1.
                            }
                        }
                    }

                    0.
                }
            }
        }
    }
}