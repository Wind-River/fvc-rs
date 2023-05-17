use libarchive::archive;
use libarchive::reader;
use libarchive::writer;

use std::path::Path;

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