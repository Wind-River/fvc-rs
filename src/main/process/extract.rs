//! extract calls libarchive to extract the given archive

use std::path::Path;
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