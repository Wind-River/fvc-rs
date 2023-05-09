// Copyright (c) 2020 Wind River Systems, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES
// OR CONDITIONS OF ANY KIND, either express or implied.

//! # file_verification_code
//! `file_verification_code` is a library to calculate a file verification code for a collection of files.
//! It is based around calculating the hash of all hashes of the included files.
//! We currently only support FVC2, which is the sha256 of sha256s.

use sha2::{Sha256, Digest};
use hex::ToHex;
use std::io::Read;

/// FVCHasher reads in data, calculates and stores its sha256, and then returns the file verification code
pub trait FVCHasher {
    /// read takes a reader, such as an open file, calculates its sha256 and stores for later output
    fn read(&mut self, reader: impl Read) -> Result<usize, std::io::Error>;
    /// sum calculates the file verification code of the currently held hashes
    fn sum(&mut self) -> Vec<u8>;
    /// hex behaves like sum, except returns the file verification code as a hex string
    fn hex(&mut self) -> String;
}

/// FVCSha256Hasher is to allow FVCHasher's whose end-goal is to calculate and store sha256s to take a sha256 directly
pub trait FVCSha256Hasher: FVCHasher {
    /// read_sha256 takes a sha256 directly and stores for later use in its FVCHasher
    fn read_sha256(&mut self, sha256: [u8; 32]);
}

/// FVC2Hasher implements File Verification Code version 2
pub struct FVC2Hasher {
    // sha256s stores the calculated sha256s until ready to calculate the file verification code
    sha256s: Vec<[u8; 32]>,
    // prevents re-sorting if sum or hex are called back-to-back
    sorted: bool,
}

impl FVC2Hasher {
    /// create a new FVC2Hasher
    pub fn new() -> Self {
        FVC2Hasher{ sha256s: Vec::new(), sorted: false}
    }
}

/// Implements FVCHasher for file verification code 2
impl FVCHasher for FVC2Hasher {
    fn read(&mut self, mut reader: impl Read) -> std::result::Result<usize, std::io::Error> {
        // calculate and store sha256 of reader
        let mut hasher = Sha256::new();
        let mut buf = Vec::new();
        match reader.read_to_end(&mut buf) {
            Ok(size) => {
                hasher.update(buf);
                self.sha256s.push(hasher.finalize().into());

                self.sorted = false; // sha256s changed and is no longer necessarily sorted
                Ok(size)
            }
            Err(e) => Err(e)
        }
    }

    fn sum(&mut self) -> Vec<u8> {
        if !self.sorted {
            // sort sha256s if necessary
            self.sha256s.sort();
            self.sorted = true;
        }

        // calculate sha256 of sorted sha256s
        let mut hasher = Sha256::new();
        for sha256 in self.sha256s.iter() {
            hasher.update(sha256);
        }

        // prepend version to final sha256
        let hash: [u8; 32] = hasher.finalize().into();
        let mut code = vec![b'F', b'V', b'C', b'2', 0];
        code.extend_from_slice(&hash[..]);

        code
    }
    fn hex(&mut self) -> String {
        // encode sum as hex string
        self.sum().encode_hex::<String>()
    }
}

// Allows FVC2Hasher to take sha256s directly
impl FVCSha256Hasher for FVC2Hasher {
    fn read_sha256(&mut self, sha256: [u8; 32]) {
        // push sha256 directly and acknowledge vector is no longer sorted
        self.sha256s.push(sha256);
        self.sorted = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::include_bytes;
    use hex_literal::hex;

    #[test]
    fn fvc2_sha256_hasher_foo_bar_zap() {
        let foo_sha256 = hex!("b5bb9d8014a0f9b1d61e21e796d78dccdf1352f23cd32812f4850b878ae4944c");
        let bar_sha256 = hex!("7d865e959b2466918c9863afca942d0fb89d7c9ac0c99bafc3749504ded97730");
        let zap_sha256 = hex!("a121b45bde6824e7ffd72c814e545a35e13b687680ea4e62a4a4405ab23acb0b");
        let sha256s = [foo_sha256, bar_sha256, zap_sha256];

        let mut hasher = FVC2Hasher::new();
        for sha256 in sha256s.iter() {
            hasher.read_sha256(*sha256)
        }

        let result = hasher.hex();
        assert_eq!(result, "4656433200ad460448a5947428e2c3e98adfe45915d71f7a4b399910fed1022cc4e1cdc374");
    }
    #[test]
    fn fvc2_hasher_foo_bar_zap() -> Result<(), std::io::Error> {
        let foo_txt = include_bytes!("../../test/foo.txt");
        let bar_txt = include_bytes!("../../test/bar.txt");
        let zap_txt = include_bytes!("../../test/zap.txt");
        let files = [foo_txt, bar_txt, zap_txt];

        let mut hasher = FVC2Hasher::new();
        for sha256 in files.iter() {
            match hasher.read(&sha256[..]) {
                Ok(_size) => (),
                Err(e) => {
                    return Err(e);
                }
            };
        }

        let result = hasher.hex();
        assert_eq!(result, "4656433200ad460448a5947428e2c3e98adfe45915d71f7a4b399910fed1022cc4e1cdc374");

        Ok(())
    }
}
