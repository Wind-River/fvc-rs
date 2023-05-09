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

use std::env;

use std::fs::metadata;

use std::fs::File;
use std::path::Path;


fn main() {
    let mut hasher = FVC2Hasher::new();
    for arg in env::args().skip(1) {
        let stat = match metadata(&arg) {
            Ok(stat) => stat,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };

        if stat.is_dir() {
            eprintln!("TODO handle directory");
        } else if stat.is_file() {
            let path = Path::new(&arg);
            eprintln!("Adding file {}", path.display());
            let file = match File::open(path) {
                Ok(file) => file,
                Err(err) => {
                    eprintln!("{}", err);
                    std::process::exit(1);
                }
            };
            match hasher.read(file) {
                Ok(_size) => {},
                Err(err) => {
                    eprintln!("{}", err);
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!("{:#?}", stat);
            std::process::exit(1);
        }
    }

    eprint!("FVC: ");
    println!("{}", hasher.hex());
}