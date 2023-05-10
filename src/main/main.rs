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
use clap::Parser;

use std::fs::File;
use std::path::PathBuf;

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
            eprint!("FVC: ");
            println!("{}", hasher.hex());        
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
        } else if stat.is_dir() {
            if cli.log_verbose() {
                eprintln!("Adding directory \"{}\"", path.display());
            }

            for entry in WalkDir::new(path).into_iter() {
                let entry = match entry {
                    Ok(dir_entry) => dir_entry,
                    Err(err) => {
                        match err.loop_ancestor() { // TODO this implies walkdir follows symlinks, can this be turned off?
                            Some(ancestor) => {
                                if cli.log_verbose() {
                                    eprintln!("Infinite loop detected at {}", ancestor.display());
                                }
                            },
                            None => ()
                        }

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