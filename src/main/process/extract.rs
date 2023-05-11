//! extract calls libarchive to extract the given archive

use std::path::Path;
use libarchive::reader::Builder;

pub enum ExtractResult {
    Ok,
    // IOError(std::io::Error),
    ArchiveError(libarchive::error::ArchiveError)
}

/// extract_archive uses libarchive to extract src to dst
pub fn extract_archive(src: &Path, dst: &Path) -> ExtractResult {
    let mut builder = Builder::default();
    builder.support_format(libarchive::archive::ReadFormat::All).expect("support all formats");
    builder.support_filter(libarchive::archive::ReadFilter::All).expect("support all filters");

    let mut reader = match builder.open_file(src) {
        Ok(reader) => reader,
        Err(err) => return ExtractResult::ArchiveError(err)
    };

    let mut opts = libarchive::archive::ExtractOptions::new();
    opts.add(libarchive::archive::ExtractOption::Time);
    opts.add(libarchive::archive::ExtractOption::ACL);
    opts.add(libarchive::archive::ExtractOption::FFlags);

    let writer = libarchive::writer::Disk::new();
    match writer.set_options(&opts) {
        Ok(()) => (),
        Err(err) => return ExtractResult::ArchiveError(err)
    };

    match writer.write(&mut reader, dst.to_str()) {
        Ok(_size) => ExtractResult::Ok,
        Err(err) => ExtractResult::ArchiveError(err)
    }
}