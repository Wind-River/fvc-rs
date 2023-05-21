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

/// FVCSha256Hasher allows sha256-based FVCHashers to take a sha256 directly instead of calculating it again
pub trait FVCSha256Hasher: FVCHasher {
    /// read_sha256 takes a sha256 directly and stores for later use in its FVCHasher
    fn read_sha256(&mut self, sha256: [u8; 32]);
}