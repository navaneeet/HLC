use crate::error::HlcError;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HlcMode {
    Balanced, // Aims for zstd-like speed
    Max,      // Prioritizes compression ratio
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChecksumType {
    CRC32,
    SHA256,
}

#[derive(Debug, Clone)]
pub struct HlcConfig {
    pub mode: HlcMode,
    pub checksum: ChecksumType,
    pub threads: usize,
    pub chunk_size: usize,
    pub entropy_level: i32,
}

impl Default for HlcConfig {
    fn default() -> Self {
        Self {
            mode: HlcMode::Balanced,
            checksum: ChecksumType::CRC32,
            threads: num_cpus::get(),
            chunk_size: 1024 * 1024, // 1 MB chunks
            entropy_level: 5,         // zstd level 5
        }
    }
}

// Allow parsing from string for CLI
impl FromStr for HlcMode {
    type Err = HlcError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "balanced" => Ok(HlcMode::Balanced),
            "max" => Ok(HlcMode::Max),
            _ => Err(HlcError::ConfigError(format!("Invalid mode: {}", s))),
        }
    }
}

impl FromStr for ChecksumType {
    type Err = HlcError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "crc32" => Ok(ChecksumType::CRC32),
            "sha256" => Ok(ChecksumType::SHA256),
            _ => Err(HlcError::ConfigError(format!("Invalid checksum type: {}", s))),
        }
    }
}

impl HlcConfig {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_mode(mut self, mode: HlcMode) -> Self {
        self.mode = mode;
        // Adjust entropy level based on mode
        self.entropy_level = match mode {
            HlcMode::Balanced => 5,
            HlcMode::Max => 19, // Maximum zstd compression
        };
        self
    }
    
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.threads = threads.max(1);
        self
    }
    
    pub fn with_checksum(mut self, checksum: ChecksumType) -> Self {
        self.checksum = checksum;
        self
    }
    
    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size.max(1024); // Minimum 1KB chunks
        self
    }
}