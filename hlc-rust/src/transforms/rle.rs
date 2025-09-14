//! Run-Length Encoding (RLE) transform
//! 
//! Compresses sequences of repeated bytes by storing the byte and its count.
//! Effective for data with many repeated values.

use anyhow::Result;
use super::Transform;

/// Run-Length Encoding transform
pub struct RLETransform {
    max_run_length: u8,
}

impl RLETransform {
    pub fn new() -> Self {
        Self {
            max_run_length: 255, // Use full byte range
        }
    }

    pub fn with_max_run_length(max_run_length: u8) -> Self {
        Self { max_run_length }
    }
}

impl Transform for RLETransform {
    fn encode(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::new();
        let mut i = 0;

        while i < data.len() {
            let current_byte = data[i];
            let mut run_length = 1;

            // Count consecutive identical bytes
            while i + run_length < data.len() 
                && data[i + run_length] == current_byte 
                && run_length < self.max_run_length as usize {
                run_length += 1;
            }

            if run_length > 1 {
                // Encode as run: [0x00, byte, count]
                result.push(0x00);
                result.push(current_byte);
                result.push(run_length as u8);
            } else {
                // Single byte: encode as literal
                if current_byte == 0x00 {
                    // Escape literal 0x00 as [0x00, 0x00, 0x01]
                    result.push(0x00);
                    result.push(0x00);
                    result.push(0x01);
                } else {
                    result.push(current_byte);
                }
            }

            i += run_length;
        }

        Ok(result)
    }

    fn decode(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::new();
        let mut i = 0;

        while i < data.len() {
            if data[i] == 0x00 {
                // Check if this is a run or escaped literal
                if i + 2 < data.len() {
                    let byte = data[i + 1];
                    let count = data[i + 2] as usize;

                    if byte == 0x00 && count == 1 {
                        // Escaped literal 0x00
                        result.push(0x00);
                    } else {
                        // Run of identical bytes
                        for _ in 0..count {
                            result.push(byte);
                        }
                    }
                    i += 3;
                } else {
                    // Malformed data, treat as literal
                    result.push(data[i]);
                    i += 1;
                }
            } else {
                // Literal byte
                result.push(data[i]);
                i += 1;
            }
        }

        Ok(result)
    }

    fn id(&self) -> u8 {
        2 // RLE transform ID
    }
}

impl Default for RLETransform {
    fn default() -> Self {
        Self::new()
    }
}

/// Optimized RLE for specific data patterns
pub struct OptimizedRLETransform {
    threshold: usize, // Minimum run length to encode
}

impl OptimizedRLETransform {
    pub fn new() -> Self {
        Self { threshold: 3 }
    }

    pub fn with_threshold(threshold: usize) -> Self {
        Self { threshold }
    }
}

impl Transform for OptimizedRLETransform {
    fn encode(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::new();
        let mut i = 0;

        while i < data.len() {
            let current_byte = data[i];
            let mut run_length = 1;

            // Count consecutive identical bytes
            while i + run_length < data.len() && data[i + run_length] == current_byte {
                run_length += 1;
            }

            if run_length >= self.threshold {
                // Encode as run: [0xFF, byte, count_low, count_high]
                result.push(0xFF);
                result.push(current_byte);
                result.push((run_length & 0xFF) as u8);
                result.push(((run_length >> 8) & 0xFF) as u8);
            } else {
                // Encode as literals
                for j in 0..run_length {
                    result.push(data[i + j]);
                }
            }

            i += run_length;
        }

        Ok(result)
    }

    fn decode(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::new();
        let mut i = 0;

        while i < data.len() {
            if data[i] == 0xFF && i + 3 < data.len() {
                // Run encoding
                let byte = data[i + 1];
                let count = (data[i + 2] as usize) | ((data[i + 3] as usize) << 8);
                
                for _ in 0..count {
                    result.push(byte);
                }
                i += 4;
            } else {
                // Literal byte
                result.push(data[i]);
                i += 1;
            }
        }

        Ok(result)
    }

    fn id(&self) -> u8 {
        3 // Optimized RLE transform ID
    }
}

impl Default for OptimizedRLETransform {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rle_roundtrip() {
        let transform = RLETransform::new();
        let original = b"AAAABBBBCCCC";
        
        let encoded = transform.encode(original).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_rle_mixed_data() {
        let transform = RLETransform::new();
        let original = b"ABCAAAABBBBCCCCDEF";
        
        let encoded = transform.encode(original).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_rle_zero_bytes() {
        let transform = RLETransform::new();
        let original = b"\x00\x00\x00\x00ABC";
        
        let encoded = transform.encode(original).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_rle_single_bytes() {
        let transform = RLETransform::new();
        let original = b"ABCDEF";
        
        let encoded = transform.encode(original).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_optimized_rle() {
        let transform = OptimizedRLETransform::with_threshold(3);
        let original = b"ABCAAAABBBBCCCCDEF";
        
        let encoded = transform.encode(original).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_empty_data() {
        let transform = RLETransform::new();
        let empty: &[u8] = &[];
        
        let encoded = transform.encode(empty).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(empty, &decoded[..]);
    }

    #[test]
    fn test_long_run() {
        let transform = RLETransform::new();
        let original = vec![b'A'; 300]; // Longer than max_run_length
        
        let encoded = transform.encode(&original).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(original, decoded);
    }
}