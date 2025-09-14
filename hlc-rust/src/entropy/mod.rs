//! Entropy coding implementations
//! 
//! Provides range coding, ANS, and integration with external compressors
//! like LZ4 and Zstd for the final compression stage.

pub mod range;
pub mod codec_utils;

use anyhow::Result;
use crate::container::EntropyMethod;

/// Trait for entropy coding implementations
pub trait EntropyCoder {
    /// Encode data using the entropy coder
    fn encode(&self, data: &[u8]) -> Result<Vec<u8>>;
    
    /// Decode data using the entropy coder
    fn decode(&self, data: &[u8]) -> Result<Vec<u8>>;
    
    /// Get the entropy method identifier
    fn method(&self) -> EntropyMethod;
}

/// Range coder implementation
pub struct RangeCoder {
    precision: u32,
}

impl RangeCoder {
    pub fn new() -> Self {
        Self { precision: 16 }
    }

    pub fn with_precision(precision: u32) -> Self {
        Self { precision }
    }
}

impl EntropyCoder for RangeCoder {
    fn encode(&self, data: &[u8]) -> Result<Vec<u8>> {
        // For now, use a simple implementation
        // In production, this would use a proper range coder
        Ok(data.to_vec())
    }

    fn decode(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn method(&self) -> EntropyMethod {
        EntropyMethod::RangeCoder
    }
}

/// ANS (Asymmetric Numeral Systems) coder
pub struct ANSCoder {
    precision: u32,
}

impl ANSCoder {
    pub fn new() -> Self {
        Self { precision: 16 }
    }

    pub fn with_precision(precision: u32) -> Self {
        Self { precision }
    }
}

impl EntropyCoder for ANSCoder {
    fn encode(&self, data: &[u8]) -> Result<Vec<u8>> {
        // For now, use a simple implementation
        // In production, this would use a proper ANS coder
        Ok(data.to_vec())
    }

    fn decode(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn method(&self) -> EntropyMethod {
        EntropyMethod::ANS
    }
}

/// LZ4 compressor wrapper
pub struct LZ4Coder {
    level: i32,
}

impl LZ4Coder {
    pub fn new() -> Self {
        Self { level: 1 }
    }

    pub fn with_level(level: i32) -> Self {
        Self { level }
    }
}

impl EntropyCoder for LZ4Coder {
    fn encode(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(lz4_flex::compress(data))
    }

    fn decode(&self, data: &[u8]) -> Result<Vec<u8>> {
        lz4_flex::decompress_size_prepended(data)
            .map_err(|e| anyhow::anyhow!("LZ4 decompression failed: {}", e))
    }

    fn method(&self) -> EntropyMethod {
        EntropyMethod::LZ4
    }
}

/// Zstd compressor wrapper
pub struct ZstdCoder {
    level: i32,
}

impl ZstdCoder {
    pub fn new() -> Self {
        Self { level: 3 }
    }

    pub fn with_level(level: i32) -> Self {
        Self { level }
    }
}

impl EntropyCoder for ZstdCoder {
    fn encode(&self, data: &[u8]) -> Result<Vec<u8>> {
        zstd::encode_all(data, self.level)
            .map_err(|e| anyhow::anyhow!("Zstd compression failed: {}", e))
    }

    fn decode(&self, data: &[u8]) -> Result<Vec<u8>> {
        zstd::decode_all(data)
            .map_err(|e| anyhow::anyhow!("Zstd decompression failed: {}", e))
    }

    fn method(&self) -> EntropyMethod {
        EntropyMethod::Zstd
    }
}

/// Entropy coder factory
pub struct EntropyCoderFactory;

impl EntropyCoderFactory {
    pub fn create(method: EntropyMethod, level: Option<i32>) -> Box<dyn EntropyCoder + Send + Sync> {
        match method {
            EntropyMethod::RangeCoder => Box::new(RangeCoder::new()),
            EntropyMethod::ANS => Box::new(ANSCoder::new()),
            EntropyMethod::LZ4 => Box::new(LZ4Coder::with_level(level.unwrap_or(1))),
            EntropyMethod::Zstd => Box::new(ZstdCoder::with_level(level.unwrap_or(3))),
            EntropyMethod::None => Box::new(NoOpCoder),
        }
    }
}

/// No-op coder for testing
pub struct NoOpCoder;

impl EntropyCoder for NoOpCoder {
    fn encode(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn decode(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn method(&self) -> EntropyMethod {
        EntropyMethod::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lz4_roundtrip() {
        let coder = LZ4Coder::new();
        let original = b"Hello, world! This is a test of LZ4 compression.";
        
        let encoded = coder.encode(original).unwrap();
        let decoded = coder.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_zstd_roundtrip() {
        let coder = ZstdCoder::new();
        let original = b"Hello, world! This is a test of Zstd compression.";
        
        let encoded = coder.encode(original).unwrap();
        let decoded = coder.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_noop_coder() {
        let coder = NoOpCoder;
        let original = b"Hello, world!";
        
        let encoded = coder.encode(original).unwrap();
        let decoded = coder.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_factory() {
        let coder = EntropyCoderFactory::create(EntropyMethod::LZ4, Some(1));
        let original = b"Test data for factory";
        
        let encoded = coder.encode(original).unwrap();
        let decoded = coder.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }
}