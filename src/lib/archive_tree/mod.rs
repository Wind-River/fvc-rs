use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs::metadata;

use log::*;
use serde::{Serialize, Deserialize};
use serde_hex::{SerHex, Strict};

#[derive(Serialize, Deserialize, PartialEq)]
pub struct File {
    pub name: String,
    pub size: u64,
    #[serde(with = "SerHex::<Strict>")]
    pub sha256: [u8; 32]
}

impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(&self) {
            Ok(s) => write!(f, "{}", s),
            Err(err) => {
                debug!("error formatting Debug File: {}", err);
                std::fmt::Result::Err(std::fmt::Error)
            }
        }
    }
}

impl std::fmt::Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string_pretty(&self) {
            Ok(s) => write!(f, "{}", s),
            Err(err) => {
                debug!("error formatting Display File: {}", err);
                std::fmt::Result::Err(std::fmt::Error)
            }
        }
    }
}

impl File {
    pub fn new<P: AsRef<Path>>(file_path: P, size: Option<u64>, sha256: Option<[u8; 32]>) -> std::io::Result<Self> {
        let size = match size {
            Some(size) => size,
            None => match metadata(&file_path) {
                Ok(metadata) => metadata.len(),
                Err(err) => return std::io::Result::Err(err)
            }
        };

        let sha256 = match sha256 {
            Some(sha256) => sha256,
            None => match get_sha256(&file_path) {
                Ok(sha256) => sha256,
                Err(err) => return std::io::Result::Err(err)
            }
        };

        let name = match file_path.as_ref().file_name() {
            Some(file_name) => file_name.to_string_lossy().into(),
            None => panic!("file has no file_name")
        };

        std::io::Result::Ok(File {
            name: name,
            size: size,
            sha256: sha256
        })
    }
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct Archive {
    pub name: String,
    pub size: u64,
    #[serde(with = "SerHex::<Strict>")]
    pub sha256: [u8; 32],
    pub files: HashMap<PathBuf, File>,
    pub archives: HashMap<PathBuf, Archive>
}

impl std::fmt::Debug for Archive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(&self) {
            Ok(s) => write!(f, "{}", s),
            Err(err) => {
                debug!("error formatting Debug Archive: {}", err);
                std::fmt::Result::Err(std::fmt::Error)
            }
        }
    }
}

impl std::fmt::Display for Archive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string_pretty(&self) {
            Ok(s) => write!(f, "{}", s),
            Err(err) => {
                debug!("error formatting Display Archive: {}", err);
                std::fmt::Result::Err(std::fmt::Error)
            }
        }
    }
}

impl Archive {
    pub fn new<P: AsRef<Path>>(source: P, size: Option<u64>, sha256: Option<[u8; 32]>) -> std::io::Result<Self> {
        let size = match size {
            Some (size) => size,
            None => match std::fs::metadata(&source) {
                Ok(metadata) => metadata.len(),
                Err(err) => {
                    debug!("source not found");
                    return Err(err);
                }
            }
        };
        let sha256 = match sha256 {
            Some(sha256) => sha256,
            None => match get_sha256(source.as_ref()) {
                Ok(sha256) => sha256,
                Err(err) => return Err(err)
            }
        };

        let name: String = match source.as_ref().file_name() {
            Some(file_name) => file_name.to_string_lossy().into(),
            None => panic!("{:?} has no file_name", source.as_ref())
        };

        Ok(Archive {
            name: name,
            size: size,
            sha256: sha256,
            files: HashMap::new(),
            archives: HashMap::new()
        })
    }

    pub fn add_file<P: AsRef<Path>>(self: &mut Self, file_path: P, size: Option<u64>, sha256: Option<[u8; 32]>) -> std::io::Result<()> {
        let file = match File::new(&file_path, size, sha256) {
            Ok(file) => file,
            Err(err) => return std::io::Result::Err(err)
        };
        
        self.files.insert(file_path.as_ref().to_owned(), file);
        Ok(())
    }

    pub fn add_archive(self: &mut Self, archive_path: PathBuf, archive: Archive) -> std::io::Result<()> {
        self.archives.insert(archive_path, archive);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct Directory {
    directory: PathBuf,
    pub files: HashMap<PathBuf, File>,
    pub archives: HashMap<PathBuf, Archive>
}

impl std::fmt::Debug for Directory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(&self) {
            Ok(s) => write!(f, "{}", s),
            Err(err) => {
                debug!("error formatting Debug Directory: {}", err);
                std::fmt::Result::Err(std::fmt::Error)
            }
        }
    }
}

impl std::fmt::Display for Directory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string_pretty(&self) {
            Ok(s) => write!(f, "{}", s),
            Err(err) => {
                debug!("error formatting Display Directory: {}", err);
                std::fmt::Result::Err(std::fmt::Error)
            }
        }
    }
}

impl Directory {
    pub fn new<P: AsRef<Path>>(directory: P) -> Self {
        Directory { directory: directory.as_ref().to_owned(), files: HashMap::new(), archives: HashMap::new() }
    }

    pub fn add_file<P: AsRef<Path>>(self: &mut Self, file_path: P, size: Option<u64>, sha256: Option<[u8; 32]>) -> std::io::Result<()> {
        let file = match File::new(&file_path, size, sha256) {
            Ok(file) => file,
            Err(err) => return std::io::Result::Err(err)
        };
        
        self.files.insert(file_path.as_ref().to_owned(), file);
        Ok(())
    }

    pub fn add_archive(self: &mut Self, archive_path: PathBuf, archive: Archive) -> std::io::Result<()> {
        self.archives.insert(archive_path, archive);
        Ok(())
    }
}

#[derive(Debug)]
pub enum Collection {
    File(File),
    Archive(Archive),
    Directory(Directory),
    Empty
}

impl serde::Serialize for Collection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        match self {
            Collection::Empty => {
                serializer.serialize_none()
            },
            Collection::File(file) => {
                file.serialize(serializer)
            },
            Collection::Archive(archive) => {
                archive.serialize(serializer)
            },
            Collection::Directory(directory)=> {
                directory.serialize(serializer)
            }
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use hex_literal::hex;

    #[test]
    fn foo_bar_zap_archive_tree() {
        let mut archive = Archive::new(
            PathBuf::from_str("./test_data/foo_bar_zap.tar.zst").unwrap(), 
            Some(132), 
            Some(hex!("c219699ccc7c7a0ff4770268bc1071664ae16c4b89cad6c3be882efd5f61c50f"))).
            expect("creating archive");
        
        archive.add_file(PathBuf::from_str("./test_data/foo_bar_zap.d/foo.txt").unwrap(), Some(4), Some(hex!("b5bb9d8014a0f9b1d61e21e796d78dccdf1352f23cd32812f4850b878ae4944c"))).expect("adding foo");
        archive.add_file(PathBuf::from_str("./test_data/foo_bar_zap.d/bar.txt").unwrap(), Some(4), Some(hex!("7d865e959b2466918c9863afca942d0fb89d7c9ac0c99bafc3749504ded97730"))).expect("adding bar");
        archive.add_file(PathBuf::from_str("./test_data/foo_bar_zap.d/zap.txt").unwrap(), Some(4), Some(hex!("a121b45bde6824e7ffd72c814e545a35e13b687680ea4e62a4a4405ab23acb0b"))).expect("adding zap");

        let serialized = serde_json::to_string_pretty(&archive).expect("serializing tree");
        let deserialized: Archive = serde_json::from_str(&serialized).expect("deserializing result");
        assert_eq!(archive, deserialized);
    }
}