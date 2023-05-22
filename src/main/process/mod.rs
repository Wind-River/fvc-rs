// Copyright (c) 2020 Wind River Systems, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES
// OR CONDITIONS OF ANY KIND, either express or implied.

use crate::FVC2Hasher;

use std::path::PathBuf;
use clap::ValueEnum;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum ExtractPolicy {
    /// Only try to extract files with extensions that look like archives
    Extension,
    /// Try to extract every file
    All,
    /// Don't extract, treat archives as binary files
    None    
}

pub trait Processor {
    fn new(extract_policy: ExtractPolicy) -> Self;
    /// calculate_fvc iterates over the given files and adds them to the FVCHasher, or extracts and/or walk given archives/directories and does the same for their files.
    /// The actual fvc at the end can be obtained from the given hasher.
    fn calculate_fvc(self: &Self, hasher: &mut FVC2Hasher, files: &[PathBuf]) -> std::io::Result<()>;
}

// use ExtractionProcessor if feature enabled
#[cfg(feature = "extract")]
mod process_extract;
#[cfg(feature = "extract")]
pub fn new(extract_policy: ExtractPolicy) -> process_extract::ExtractionProcessor {
    process_extract::ExtractionProcessor::new(extract_policy)
}
#[cfg(feature = "extract")]
pub fn default_policy() -> ExtractPolicy {
    ExtractPolicy::Extension
}

// use SimpleProcessor that treats archives as files if extraction feature disabled
#[cfg(not(feature = "extract"))]
mod process;
#[cfg(not(feature = "extract"))]
pub fn new(extract_policy: ExtractPolicy) -> process::SimpleProcessor {
    process::SimpleProcessor::new(extract_policy)
}
#[cfg(not(feature = "extract"))]
pub fn default_policy() -> ExtractPolicy {
    ExtractPolicy::None
}
