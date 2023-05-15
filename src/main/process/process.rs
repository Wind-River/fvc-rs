use crate::FVC2Hasher;
use super::{ExtractPolicy, Processor, process_file};

use std::path::PathBuf;

use walkdir::WalkDir;
use std::fs::metadata;
use log::info;


pub struct SimpleProcessor {}
impl Processor for SimpleProcessor {
    fn new(extract_policy: ExtractPolicy) -> Self {
        assert_eq!(extract_policy, ExtractPolicy::None);
        Self {  }
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
                match process_file(hasher, path) {
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
                        match process_file(hasher, entry.path()) {
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