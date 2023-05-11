// Copyright (c) 2020 Wind River Systems, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES
// OR CONDITIONS OF ANY KIND, either express or implied.

//! `fvc` is a utility that will collect all the files it is given and calculate a file verification code of all of them

// local imports
mod process;
use process::calculate_fvc;
use file_verification_code::FVCHasher;
use file_verification_code::FVC2Hasher;
// external imports
use std::io::Write;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version=None)] // causes version to be read from Cargo.toml
#[command(about="Calculate file verification code of given files")]
pub struct CLI {
    #[arg(short='b', long="binary", help="Output FVC in binary form instead of hex-encoded string")]
    binary_mode: bool,
    #[arg(short='v', long="verbose", help="Include more v's for higher verbosity", action=clap::ArgAction::Count)]
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

    // traverse given files and calculate file verification code of all of them
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