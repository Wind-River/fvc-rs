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

mod process;
use process::{calculate_fvc, ExtractPolicy};
use file_verification_code::FVCHasher;
use file_verification_code::FVC2Hasher;

use std::io::Write;
use clap::Parser;
use std::path::PathBuf;
use log::{debug};
use colored::Colorize;

#[derive(Parser, Debug)]
#[command(version)] // causes version to be read from Cargo.toml
#[command(disable_version_flag=true)] // since we use v for verbosity, we need to manually define the version flag
#[command(about="Calculate file verification code of given files")]
#[command(after_help=get_examples())]
#[command(arg_required_else_help=true)]
struct CLI {
    #[arg(long, action=clap::ArgAction::Version)] // manually define --version flag since we are using v for verbosity
    #[arg(short='e', long="examples", action=clap::ArgAction::SetTrue)]
    show_examples: bool,
    // since neither -h nor --help are in use, help arg is auto-generated

    #[arg(short='v', long="verbose", help="Include more v's for higher verbosity", action=clap::ArgAction::Count)]
    verbose: u8,
    #[arg(short='b', long="binary", help="Output FVC in binary form instead of hex-encoded string")]
    binary_mode: bool,
    #[arg(short, long, help="Output to given file")]
    output: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t=ExtractPolicy::Extension, help="How to decide what files to try extracting")]
    extract: ExtractPolicy, 
    #[arg(help="Files or directory of files to calculate file verification code of")]
    files: Vec<PathBuf>,
}

fn get_examples() -> String {
    format!(r#"{header}
Calculate File Verification Code of all text files in a directory
    {prompt}fvc src/main/test_data/*.txt
    FVC: 4656433200ad460448a5947428e2c3e98adfe45915d71f7a4b399910fed1022cc4e1cdc374

Calculate File Verification Code of a directory's contents
    {prompt}fvc src/main/test_data/
    FVC: 465643320080906dab16c118543c5b8ce2f5a819ae1e690b992e04f5f61f73f1886a3037ba

Calculate File Verification Code of all text files under a directory and of a directory's contents
    {prompt}fvc src/main/test_data/*.txt src/main/test_data/A/
    FVC: 4656433200739776f8fabb193aa2b9df1579e27e42453164bd519d17f191e02a7485a35b96

Calculate File Verification Code of an archive
    {prompt}fvc src/main/test_data/foo_bar_zap.tar.gz
    FVC: 4656433200ad460448a5947428e2c3e98adfe45915d71f7a4b399910fed1022cc4e1cdc374

Write a binary File Verification Code to a file
    {prompt}fvc -b -o /tmp/fvc src/main/test_data/*.txt

Redirect a binary File Verification Code to a file
    {prompt}fvc -b src/main/test_data/*.txt > /tmp/fvc
    "#, 
    header="Examples:".bold().underline(),
    prompt="> ".bold())
}

fn main() {
    let cli = CLI::parse(); // parse command line

    // initialize logger
    stderrlog::new()
        .module(module_path!())
        .verbosity(match cli.verbose {
            0 => log::Level::Warn, // Start with Error and Warn
            1 => log::Level::Info,
            2 => log::Level::Debug,
            _ => log::Level::Trace // 3 or higher
        })
        .timestamp(match cli.verbose {
            0 | 1 => stderrlog::Timestamp::Off,
            2 => stderrlog::Timestamp::Second,
            _ => stderrlog::Timestamp::Millisecond // 3 or higher
        })
        .init()
        .expect("initializing logger");

    if cli.show_examples {
        // print examples and exit
        eprintln!("{}", get_examples());
        std::process::exit(0);
    }

    debug!("CLI: {:?}", cli);

    // traverse given files and calculate file verification code of all of them
    let mut hasher = FVC2Hasher::new();
    calculate_fvc(&mut hasher, cli.extract, &cli.files[..]).expect("processing given files");

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