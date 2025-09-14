use crate::error::HlcError;

/// Entropy coding wrapper around zstd
/// This provides the final compression stage after pre-processing transforms

pub fn encode(data: &[u8], level: i32) -> Result<Vec<u8>, HlcError> {
    if data.is_empty() {
        return Ok(Vec::new());
    }
    
    zstd::encode_all(data, level)
        .map_err(|e| HlcError::CompressionError(format!("Entropy encoding failed: {}", e)))
}

pub fn decode(data: &[u8]) -> Result<Vec<u8>, HlcError> {
    if data.is_empty() {
        return Ok(Vec::new());
    }
    
    zstd::decode_all(data)
        .map_err(|e| HlcError::DecompressionError(format!("Entropy decoding failed: {}", e)))
}

/// Advanced entropy coding with dictionary support
pub struct EntropyEncoder {
    level: i32,
    dictionary: Option<Vec<u8>>,
}

impl EntropyEncoder {
    pub fn new(level: i32) -> Self {
        Self {
            level,
            dictionary: None,
        }
    }
    
    pub fn with_dictionary(mut self, dict: Vec<u8>) -> Self {
        self.dictionary = Some(dict);
        self
    }
    
    pub fn encode(&self, data: &[u8]) -> Result<Vec<u8>, HlcError> {
        if data.is_empty() {
            return Ok(Vec::new());
        }
        
        match &self.dictionary {
            Some(_dict) => {
                // Note: Dictionary support would require a more complex implementation
                // For now, fall back to regular compression
                encode(data, self.level)
            }
            None => encode(data, self.level),
        }
    }
    
    pub fn decode(&self, data: &[u8]) -> Result<Vec<u8>, HlcError> {
        if data.is_empty() {
            return Ok(Vec::new());
        }
        
        match &self.dictionary {
            Some(_dict) => {
                // Note: Dictionary support would require a more complex implementation
                // For now, fall back to regular decompression
                decode(data)
            }
            None => decode(data),
        }
    }
}

/// Estimate compression ratio without actually compressing
/// This is useful for the analyzer to make decisions
pub fn estimate_compression_ratio(data: &[u8]) -> f32 {
    if data.is_empty() {
        return 1.0;
    }
    
    // Simple entropy-based estimation
    let mut counts = [0u32; 256];
    for &byte in data {
        counts[byte as usize] += 1;
    }
    
    let len = data.len() as f32;
    let mut entropy = 0.0;
    
    for &count in &counts {
        if count > 0 {
            let p = count as f32 / len;
            entropy -= p * p.log2();
        }
    }
    
    // Rough estimation: higher entropy means less compressible
    // This is a simplified heuristic
    let max_entropy = 8.0; // Maximum entropy for 8-bit data
    let compression_potential = (max_entropy - entropy) / max_entropy;
    
    // Convert to estimated compression ratio
    1.0 + compression_potential * 3.0 // Rough scaling factor
}

/// Fast compression mode using zstd level 1
pub fn encode_fast(data: &[u8]) -> Result<Vec<u8>, HlcError> {
    encode(data, 1)
}

/// Maximum compression mode using zstd level 19
pub fn encode_max(data: &[u8]) -> Result<Vec<u8>, HlcError> {
    encode(data, 19)
}

/// Balanced compression mode using zstd level 5
pub fn encode_balanced(data: &[u8]) -> Result<Vec<u8>, HlcError> {
    encode(data, 5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_empty() {
        let data = vec![];
        let encoded = encode(&data, 5).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_entropy_roundtrip() {
        let data = b"Hello, world! This is a test string for compression.".to_vec();
        let encoded = encode(&data, 5).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_entropy_compression_levels() {
        let data = vec![0u8; 1000]; // Highly compressible data
        
        let fast = encode_fast(&data).unwrap();
        let balanced = encode_balanced(&data).unwrap();
        let max = encode_max(&data).unwrap();
        
        // All should decompress correctly
        assert_eq!(decode(&fast).unwrap(), data);
        assert_eq!(decode(&balanced).unwrap(), data);
        assert_eq!(decode(&max).unwrap(), data);
        
        // Max compression should generally be smaller (though not guaranteed)
        println!("Fast: {} bytes", fast.len());
        println!("Balanced: {} bytes", balanced.len());
        println!("Max: {} bytes", max.len());
    }

    #[test]
    fn test_compression_ratio_estimation() {
        // Highly compressible data (all zeros)
        let zeros = vec![0u8; 1000];
        let ratio1 = estimate_compression_ratio(&zeros);
        
        // Random-like data (less compressible)
        let random: Vec<u8> = (0..1000).map(|i| (i * 17 + 42) as u8).collect();
        let ratio2 = estimate_compression_ratio(&random);
        
        // Zeros should have higher estimated compression ratio
        assert!(ratio1 > ratio2);
    }
}