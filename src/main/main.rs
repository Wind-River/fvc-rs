// Copyright (c) 2020 Wind River Systems, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES
// OR CONDITIONS OF ANY KIND, either express or implied.

use file_verification_code::FVCHasher;
use file_verification_code::FVC2Hasher;
use walkdir::WalkDir;

use std::fs::metadata;
use std::io::Write;
use std::path::Path;
use clap::Parser;

use std::fs::File;
use std::path::PathBuf;

use libarchive::reader::Builder;

#[derive(Parser, Debug)]
#[command(version=None)] // causes version to be read from Cargo.toml
#[command(about="Calculate file verification code of given files")]
struct CLI {
    #[arg(short='b', long="binary")]
    binary_mode: bool,
    #[arg(short='v', long="verbose", action=clap::ArgAction::Count)]
    verbose: u8,
    #[arg(short='e', long="examples")]
    show_examples: bool,
    #[arg(short, long, help="Output to given file")]
    output: Option<PathBuf>,
    #[arg(required=true, help="Files or directory of files to calculate file verification code of")]
    files: Vec<PathBuf>,
}

impl CLI {
    pub fn log_verbose(&self) -> bool {
        self.verbose > 0
    }
    pub fn log_debug(&self) -> bool {
        self.verbose > 1
    }
}

fn main() {
    let cli = CLI::parse();

    if cli.show_examples {
        eprintln!("TODO show examples");
        std::process::exit(0);
    }

    if cli.log_debug() {
        eprintln!("CLI: {:?}", cli);
    }

    let mut hasher = FVC2Hasher::new();
    calculate_fvc(&cli, &mut hasher, &cli.files[..]).expect("processing given files");

    match cli.output {
        Some(path) => {
            // Write to file
            if cli.binary_mode {
                std::fs::write(&path, hasher.sum()).expect("writing binary fvc to file");
            } else {
                std::fs::write(&path, hasher.hex()).expect("writing hex fvc to file");
            }
        },
        None => {
            // Print to stdout
            if cli.binary_mode {
                std::io::stdout().write_all(&hasher.sum()[..]).expect("writing binary to stdout");
            } else {
                eprint!("FVC: ");
                println!("{}", hasher.hex());        
            }
        }
    }
}

fn calculate_fvc(cli: &CLI, hasher: &mut FVC2Hasher, files: &[PathBuf]) -> std::io::Result<()> {
    for path in files {
        let stat = match metadata(path) {
            Ok(metadata) => metadata,
            Err(err) => {
                return Err(err);
            }
        };

        if stat.is_file() {
            if is_archive(path) {
                eprintln!("TODO extract archive");
                let tmp = match tempdir::TempDir::new("") {
                    Ok(tmp) => tmp,
                    Err(err) => return Err(err)
                };
                match extract_archive(&path, tmp.path()) {
                    ExtractResult::Ok => {
                        if cli.log_verbose() {
                            eprintln!("extracted archive {}", path.display());
                        }
                        match calculate_fvc(&cli, hasher, &[tmp.into_path()]) {
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

fn is_archive(path: &Path) -> bool {
    match path.extension() {
        None => false,
        Some(ext) => {
            match ext.to_str() {
                None => false,
                Some("zip") => true,
                Some("gz")=> true,
                Some("bz2") => true,
                Some("xz") => true,
                Some(_) => false
            }
        }
    }
}

enum ExtractResult {
    Ok,
    // IOError(std::io::Error),
    ArchiveError(libarchive::error::ArchiveError)
}

fn extract_archive(src: &Path, dst: &Path) -> ExtractResult {
    let mut builder = Builder::default();
    builder.support_format(libarchive::archive::ReadFormat::All).expect("support all formats");
    builder.support_filter(libarchive::archive::ReadFilter::All).expect("support all filters");
    // builder.support_format(libarchive::archive::ReadFormat::Tar).expect("support tar");
    // builder.support_format(libarchive::archive::ReadFormat::Zip).expect("support zip");
    // builder.support_filter(libarchive::archive::ReadFilter::Gzip).expect("support gz");
    // // builder.support_filter(libarchive::archive::ReadFilter::Bzip2).expect("support bz2");
    // builder.support_filter(libarchive::archive::ReadFilter::Xz).expect("suppor xz");

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

    // match FileReader::open(builder, src) {
    //     Ok(mut reader) => {
    //         while let Some(entry) = reader.next_header() {
    //             let target = dst.join(entry.pathname());
    //             let mut file = match File::create(&target) {
    //                 Ok(file) => file,
    //                 Err(err) => return ExtractResult::IOError(err),
    //             };

    //             // entry.set_pathname(&target);

    //             match reader.read_block() {
    //                 Ok(Some(payload)) => {
    //                     match file.write_all(payload) {
    //                         Ok(()) => (),
    //                         Err(err) => return ExtractResult::IOError(err),
    //                     }
    //                 },
    //                 Ok(None) => (),
    //                 Err(err) => return ExtractResult::ArchiveError(err),
    //             };
    //         }
    //         ExtractResult::Ok
    //     },
    //     Err(err) => ExtractResult::ArchiveError(err)
    // }
}