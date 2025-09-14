use crate::error::HlcError;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HlcMode {
    Balanced,
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumType {
    CRC32,
    SHA256,
}

#[derive(Debug, Clone)]
pub struct HlcConfig {
    pub mode: HlcMode,
    pub checksum: ChecksumType,
    pub threads: usize,
}

impl Default for HlcConfig {
    fn default() -> Self {
        Self {
            mode: HlcMode::Balanced,
            checksum: ChecksumType::CRC32,
            threads: num_cpus::get(),
        }
    }
}

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

