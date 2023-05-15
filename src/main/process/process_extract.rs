use super::{ExtractPolicy, Processor, process_file};
use crate::FVC2Hasher;

use std::path::{Path, PathBuf};
use std::fs::metadata;
use log::{warn, info, trace};
use walkdir::WalkDir;


pub struct ExtractionProcessor {
    extract_policy: ExtractPolicy
}

impl Processor for ExtractionProcessor {
    fn new(extract_policy: ExtractPolicy) -> Self {
        Self { extract_policy: extract_policy }
    }

    fn calculate_fvc(self: &Self, hasher: &mut FVC2Hasher, files: &[PathBuf]) -> std::io::Result<()> {
        for path in files {
            let stat = match metadata(path) {
                Ok(metadata) => metadata,
                Err(err) => {
                    return Err(err);
                }
            };
    
            if stat.is_file() {
                match self.extract_or_process_file(hasher, self.extract_policy, path) {
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
    
                    // only process files
                    if entry.file_type().is_file() {
                        match self.extract_or_process_file(hasher, self.extract_policy, entry.path()) {
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
    
        Ok(())
    }
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
        match extract_archive(&archive_path, tmp.path()) {
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
    fn extract_or_process_file(self: &Self, hasher: &mut FVC2Hasher, extract_policy: ExtractPolicy, file_path: &Path) -> std::io::Result<()> {
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
                match self.process_archive(hasher, extract_policy, file_path) {
                    Ok(()) => Ok(()),
                    Err(err) => Err(err)
                }
            },
            (ExtractPolicy::All, _) => {
                // Try processing every archive
                match self.process_archive(hasher, extract_policy, file_path) {
                    Ok(()) => Ok(()),
                    Err(err) => Err(err)
                }
            },
        }
    }
}

use libarchive::archive;
use libarchive::reader;
use libarchive::writer;

/// extract_archive uses libarchive to extract src to dst
pub fn extract_archive(src: &Path, dst: &Path) -> Result<(), libarchive::error::ArchiveError> {
    let mut builder = reader::Builder::default();
    builder.support_format(archive::ReadFormat::All).expect("support all formats");
    builder.support_filter(archive::ReadFilter::All).expect("support all filters");

    let mut reader = match builder.open_file(src) {
        Ok(reader) => reader,
        Err(err) => return Err(err)
    };

    let writer = writer::Disk::new();

    match writer.write(&mut reader, dst.to_str()) {
        Ok(_size) => Ok(()),
        Err(err) => Err(err)
    }
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
                    if s == "pack" { // If is a git pack file instead of pack200 file, it is not an archive
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