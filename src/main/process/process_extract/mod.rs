use self::extract::is_extractable;

use super::{ExtractPolicy, Processor, process_file};
use crate::FVC2Hasher;
mod dag;
use dag::{ArchiveGraph, EdgeResult};
mod extract;

use std::path::{Path, PathBuf};
use std::fs::metadata;
use log::{warn, error, info, debug, trace};
use walkdir::WalkDir;
use hex::ToHex;


pub struct ExtractionProcessor {
    extract_policy: ExtractPolicy,
}

impl Processor for ExtractionProcessor {
    fn new(extract_policy: ExtractPolicy) -> Self {
        Self { extract_policy: extract_policy }
    }

    fn calculate_fvc(self: &Self, hasher: &mut FVC2Hasher, files: &[PathBuf]) -> std::io::Result<()> {
        for path in files {
            match self.calculate_fvc_of(&mut dag::ArchiveGraph::new(), None, hasher, path) {
                Ok(()) => (),
                Err(err) => return Err(err)
            }
        }
    
        Ok(())
    }
}

enum FileResult {
    Ok,
    File,
    ExtractedArchive([u8; 32], PathBuf),
    Error(std::io::Error),
}

impl ExtractionProcessor {
    // process_archive tries to extract the given archive and process its contents
    // if it fails to extract it is passed off to process_file
    fn process_archive(self: &Self, hasher: &mut FVC2Hasher, extract_policy: ExtractPolicy, archive_path: &Path) -> std::io::Result<()> {
        let tmp_prefix = match archive_path.file_name() {
            Some(file_name) => format!("fvc_extracted_archive.{:?}", file_name),
            None => format!("fvc_extracted_archive.{:?}", archive_path)
        };
        let tmp = match tempdir::TempDir::new(&tmp_prefix) {
            Ok(tmp) => tmp,
            Err(err) => return Err(err)
        };
        match extract::extract_archive(&archive_path, tmp.path()) {
            Ok(()) => {
                info!("extracted archive {}", archive_path.display());
                match self.calculate_fvc(hasher, &[tmp.path().to_owned()]) {
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

    // extract_or_process_file looks at a path and applies the given extraction policy
    // On the extremes ExtractPolicy::None and ExtractPolicy::All will always or never process a path as an archive
    // ExtractPolicy::Extension will look at the file extension and extract it if it looks like an archive, otherwise it will process it as a file
    // In every case, if an archive fails to extract, it is treated as a file
    //self: &Self, graph: &mut ArchiveGraph, current: Option<[u8; 32]>, hasher: &mut FVC2Hasher, filepath: &Path
    fn extract_or_process_file(self: &Self, graph: &mut ArchiveGraph, current: Option<[u8; 32]>, hasher: &mut FVC2Hasher, file_path: &Path) -> std::io::Result<()> {
        match self.extract_policy {
            ExtractPolicy::None => return process_file(hasher, file_path),
            ExtractPolicy::All | ExtractPolicy::Extension => {
                let sha256 = match get_sha256(file_path) {
                    Ok(sha256) => sha256,
                    Err(err) => return Err(err)
                };
                let known_archive = ArchiveGraph::contains(graph, sha256);
                match (known_archive, current) {
                    (true, Some(current)) => {
                        // check for cycle
                        match graph.add_edge(current, sha256) {
                            EdgeResult::Ok => (),
                            EdgeResult::CycleDetected => return Ok(()), // exit early to avoid cycle
                            EdgeResult::KeyMissing(key) => panic!("Key missing for known archive? {}", key.encode_hex::<String>())
                        };

                        // extract and process directory
                        match open_archive(file_path) {
                            ExtractResult::Ok(extracted_directory) => {
                                match self.calculate_fvc_of(graph, Some(sha256), hasher, extracted_directory.path()) {
                                    Ok(()) => {
                                        match extracted_directory.close() { // clean up extraction
                                            Ok(()) => return Ok(()),
                                            Err(err) => return Err(err)
                                        };
                                    },
                                    Err(err) => return Err(err)
                                }
                            },
                            ExtractResult::IOError(err) => return Err(err),
                            ExtractResult::ArchiveError(err) => panic!("archive error for known archive: {}", err)
                        }

                    },
                    (true, None) => {
                        // no cycle possible
                        // extract and process directory
                        match open_archive(file_path) {
                            ExtractResult::Ok(extracted_directory) => {
                                match self.calculate_fvc_of(graph, Some(sha256), hasher, extracted_directory.path()) {
                                    Ok(()) => {
                                        match extracted_directory.close() { // clean up extraction
                                            Ok(()) => return Ok(()),
                                            Err(err) => return Err(err)
                                        };
                                    },
                                    Err(err) => return Err(err)
                                }
                            },
                            ExtractResult::IOError(err) => return Err(err),
                            ExtractResult::ArchiveError(err) => panic!("archive error for known archive: {}", err)
                        }
                    }
                    (false, _) => ()
                };

                // unknown if archive or file
                match (self.extract_policy, extract::is_extractable(file_path)) {
                    (ExtractPolicy::Extension, 0) => (),
                    (_, 100) => {
                        match open_archive(file_path) {
                            ExtractResult::IOError(err) => return Err(err),
                            ExtractResult::ArchiveError(_err) => {
                                debug!("Error extracting 100 confidence archive: {}", file_path.display());
                            },
                            ExtractResult::Ok(extracted_directory) => {
                                graph.insert(sha256);
                                match self.calculate_fvc_of(graph, Some(sha256), hasher, extracted_directory.path()) {
                                    Ok(()) => {
                                        match extracted_directory.close() { // clean up extraction
                                            Ok(()) => return Ok(()),
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
                        match open_archive(file_path) {
                            ExtractResult::IOError(err) => return Err(err),
                            ExtractResult::ArchiveError(_err) => (),
                            ExtractResult::Ok(extracted_directory) => {
                                graph.insert(sha256);
                                match self.calculate_fvc_of(graph, Some(sha256), hasher, extracted_directory.path()) {
                                    Ok(()) => {
                                        match extracted_directory.close() { // clean up extraction
                                            Ok(()) => return Ok(()),
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
                return process_file(hasher, file_path);
            }
        }
    }

    fn calculate_fvc_of(self: &Self, graph: &mut ArchiveGraph, current: Option<[u8; 32]>, hasher: &mut FVC2Hasher, filepath: &Path) -> std::io::Result<()> {
        let stat = match metadata(filepath) {
            Ok(metadata) => metadata,
            Err(err) => {
                return Err(err);
            }
        };

        if stat.is_file() {
            return self.extract_or_process_file(graph, current, hasher, filepath);            
        } else if stat.is_dir() {
            info!("Adding directory \"{}\"", filepath.display());

            for entry in WalkDir::new(filepath) {
                let entry = match entry {
                    Ok(dir_entry) => dir_entry,
                    Err(err) => {
                        return Err(err.into()); // walkdir::Error is a light wrapper around std::io::Error
                    }
                };
                trace!("at entry {}", entry.path().display());

                // only process files
                if entry.file_type().is_file() {
                    match self.extract_or_process_file(graph, current, hasher, entry.path()) {
                        Ok(()) => (),
                        Err(err) => return Err(err)
                    }
                }
            }
        } else {
            info!("Skipping irregular file {}", filepath.display());
        }
    
        Ok(())
    }
}

enum ExtractResult {
    Ok(tempdir::TempDir),
    IOError(std::io::Error),
    ArchiveError(libarchive::error::ArchiveError)
}

fn get_sha256(path: &Path) -> std::io::Result<[u8; 32]> {
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

fn open_archive(archive_path: &Path) -> ExtractResult {
    let tmp_prefix = match archive_path.file_name() {
        Some(file_name) => format!("fvc_extracted_archive.{:?}", file_name),
        None => format!("fvc_extracted_archive.{:?}", archive_path)
    };
    let tmp = match tempdir::TempDir::new(&tmp_prefix) {
        Ok(tmp) => tmp,
        Err(err) => return ExtractResult::IOError(err)
    };

    match extract::extract_archive(&archive_path, tmp.path()) {
        Ok(()) => {
            info!("extracted archive {}", archive_path.display());
            ExtractResult::Ok(tmp)
        },
        Err(err) => {
            match tmp.close() { // explicitly clean-up tmp directory to be able to log errors
                Ok(()) => (),
                Err(err) => debug!("error closing tmp directory: {}", err)
            };
            ExtractResult::ArchiveError(err)
        }
    }
}