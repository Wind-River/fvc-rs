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

mod fvc_hasher;
pub use fvc_hasher::{FVCHasher, FVCSha256Hasher};

mod version_2;
pub use version_2::FVC2Hasher;

#[cfg(feature = "extract")]
pub mod extract;
#[cfg(feature = "extract")]
pub mod archive_tree;