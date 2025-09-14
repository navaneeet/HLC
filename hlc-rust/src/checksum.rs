//! Checksum and hash functions for data integrity
//! 
//! Provides CRC32 and SHA-256 implementations for verifying data integrity
//! during compression and decompression.

use anyhow::Result;
use crc32fast::Hasher as Crc32Hasher;
use sha2::{Sha256, Digest};

/// Checksum types supported by HLC
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumType {
    Crc32,
    Sha256,
}

/// Checksum calculator
pub struct ChecksumCalculator {
    checksum_type: ChecksumType,
}

impl ChecksumCalculator {
    pub fn new(checksum_type: ChecksumType) -> Self {
        Self { checksum_type }
    }

    /// Calculate checksum for data
    pub fn calculate(&self, data: &[u8]) -> Result<ChecksumResult> {
        match self.checksum_type {
            ChecksumType::Crc32 => self.calculate_crc32(data),
            ChecksumType::Sha256 => self.calculate_sha256(data),
        }
    }

    /// Verify data against a checksum
    pub fn verify(&self, data: &[u8], expected: &ChecksumResult) -> Result<bool> {
        let calculated = self.calculate(data)?;
        Ok(calculated == *expected)
    }

    fn calculate_crc32(&self, data: &[u8]) -> Result<ChecksumResult> {
        let mut hasher = Crc32Hasher::new();
        hasher.update(data);
        let crc32 = hasher.finalize();
        Ok(ChecksumResult::Crc32(crc32))
    }

    fn calculate_sha256(&self, data: &[u8]) -> Result<ChecksumResult> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        Ok(ChecksumResult::Sha256(hash.into()))
    }
}

/// Result of a checksum calculation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChecksumResult {
    Crc32(u32),
    Sha256([u8; 32]),
}

impl ChecksumResult {
    /// Get the checksum type
    pub fn checksum_type(&self) -> ChecksumType {
        match self {
            ChecksumResult::Crc32(_) => ChecksumType::Crc32,
            ChecksumResult::Sha256(_) => ChecksumType::Sha256,
        }
    }

    /// Get the checksum as bytes
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            ChecksumResult::Crc32(crc) => crc.to_le_bytes().to_vec(),
            ChecksumResult::Sha256(hash) => hash.to_vec(),
        }
    }

    /// Create from bytes
    pub fn from_bytes(checksum_type: ChecksumType, bytes: &[u8]) -> Result<Self> {
        match checksum_type {
            ChecksumType::Crc32 => {
                if bytes.len() != 4 {
                    return Err(anyhow::anyhow!("Invalid CRC32 length"));
                }
                let mut crc_bytes = [0u8; 4];
                crc_bytes.copy_from_slice(bytes);
                Ok(ChecksumResult::Crc32(u32::from_le_bytes(crc_bytes)))
            }
            ChecksumType::Sha256 => {
                if bytes.len() != 32 {
                    return Err(anyhow::anyhow!("Invalid SHA256 length"));
                }
                let mut hash_bytes = [0u8; 32];
                hash_bytes.copy_from_slice(bytes);
                Ok(ChecksumResult::Sha256(hash_bytes))
            }
        }
    }

    /// Get the size of the checksum in bytes
    pub fn size(&self) -> usize {
        match self {
            ChecksumResult::Crc32(_) => 4,
            ChecksumResult::Sha256(_) => 32,
        }
    }
}

/// Streaming checksum calculator for large data
pub struct StreamingChecksumCalculator {
    checksum_type: ChecksumType,
    crc32_hasher: Option<Crc32Hasher>,
    sha256_hasher: Option<Sha256>,
}

impl StreamingChecksumCalculator {
    pub fn new(checksum_type: ChecksumType) -> Self {
        Self {
            checksum_type,
            crc32_hasher: match checksum_type {
                ChecksumType::Crc32 => Some(Crc32Hasher::new()),
                ChecksumType::Sha256 => None,
            },
            sha256_hasher: match checksum_type {
                ChecksumType::Crc32 => None,
                ChecksumType::Sha256 => Some(Sha256::new()),
            },
        }
    }

    /// Update the checksum with new data
    pub fn update(&mut self, data: &[u8]) {
        match self.checksum_type {
            ChecksumType::Crc32 => {
                if let Some(ref mut hasher) = self.crc32_hasher {
                    hasher.update(data);
                }
            }
            ChecksumType::Sha256 => {
                if let Some(ref mut hasher) = self.sha256_hasher {
                    hasher.update(data);
                }
            }
        }
    }

    /// Finalize the checksum calculation
    pub fn finalize(self) -> Result<ChecksumResult> {
        match self.checksum_type {
            ChecksumType::Crc32 => {
                if let Some(hasher) = self.crc32_hasher {
                    Ok(ChecksumResult::Crc32(hasher.finalize()))
                } else {
                    Err(anyhow::anyhow!("CRC32 hasher not initialized"))
                }
            }
            ChecksumType::Sha256 => {
                if let Some(hasher) = self.sha256_hasher {
                    let hash = hasher.finalize();
                    Ok(ChecksumResult::Sha256(hash.into()))
                } else {
                    Err(anyhow::anyhow!("SHA256 hasher not initialized"))
                }
            }
        }
    }
}

/// Utility functions for checksum operations
pub struct ChecksumUtils;

impl ChecksumUtils {
    /// Calculate CRC32 for data
    pub fn crc32(data: &[u8]) -> u32 {
        let mut hasher = Crc32Hasher::new();
        hasher.update(data);
        hasher.finalize()
    }

    /// Calculate SHA-256 for data
    pub fn sha256(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }

    /// Verify CRC32
    pub fn verify_crc32(data: &[u8], expected: u32) -> bool {
        Self::crc32(data) == expected
    }

    /// Verify SHA-256
    pub fn verify_sha256(data: &[u8], expected: &[u8; 32]) -> bool {
        Self::sha256(data) == *expected
    }

    /// Calculate both CRC32 and SHA-256
    pub fn calculate_both(data: &[u8]) -> (u32, [u8; 32]) {
        (Self::crc32(data), Self::sha256(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32_calculation() {
        let calculator = ChecksumCalculator::new(ChecksumType::Crc32);
        let data = b"Hello, world!";
        let result = calculator.calculate(data).unwrap();
        
        match result {
            ChecksumResult::Crc32(crc) => {
                assert_eq!(crc, ChecksumUtils::crc32(data));
            }
            _ => panic!("Expected CRC32 result"),
        }
    }

    #[test]
    fn test_sha256_calculation() {
        let calculator = ChecksumCalculator::new(ChecksumType::Sha256);
        let data = b"Hello, world!";
        let result = calculator.calculate(data).unwrap();
        
        match result {
            ChecksumResult::Sha256(hash) => {
                assert_eq!(hash, ChecksumUtils::sha256(data));
            }
            _ => panic!("Expected SHA256 result"),
        }
    }

    #[test]
    fn test_checksum_verification() {
        let calculator = ChecksumCalculator::new(ChecksumType::Crc32);
        let data = b"Hello, world!";
        let result = calculator.calculate(data).unwrap();
        
        assert!(calculator.verify(data, &result).unwrap());
        
        let wrong_data = b"Hello, world?";
        assert!(!calculator.verify(wrong_data, &result).unwrap());
    }

    #[test]
    fn test_streaming_calculator() {
        let mut calculator = StreamingChecksumCalculator::new(ChecksumType::Crc32);
        calculator.update(b"Hello, ");
        calculator.update(b"world!");
        
        let result = calculator.finalize().unwrap();
        let expected = ChecksumUtils::crc32(b"Hello, world!");
        
        match result {
            ChecksumResult::Crc32(crc) => assert_eq!(crc, expected),
            _ => panic!("Expected CRC32 result"),
        }
    }

    #[test]
    fn test_checksum_result_serialization() {
        let data = b"Test data";
        let crc32 = ChecksumUtils::crc32(data);
        let sha256 = ChecksumUtils::sha256(data);
        
        let crc_result = ChecksumResult::Crc32(crc32);
        let sha_result = ChecksumResult::Sha256(sha256);
        
        // Test serialization
        let crc_bytes = crc_result.as_bytes();
        let sha_bytes = sha_result.as_bytes();
        
        assert_eq!(crc_bytes.len(), 4);
        assert_eq!(sha_bytes.len(), 32);
        
        // Test deserialization
        let crc_deserialized = ChecksumResult::from_bytes(ChecksumType::Crc32, &crc_bytes).unwrap();
        let sha_deserialized = ChecksumResult::from_bytes(ChecksumType::Sha256, &sha_bytes).unwrap();
        
        assert_eq!(crc_result, crc_deserialized);
        assert_eq!(sha_result, sha_deserialized);
    }

    #[test]
    fn test_checksum_size() {
        let crc_result = ChecksumResult::Crc32(0x12345678);
        let sha_result = ChecksumResult::Sha256([0u8; 32]);
        
        assert_eq!(crc_result.size(), 4);
        assert_eq!(sha_result.size(), 32);
    }
}