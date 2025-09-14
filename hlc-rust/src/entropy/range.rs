//! Range coder implementation
//! 
//! A simplified range coder for entropy coding. This is a basic implementation
//! that can be extended with more sophisticated features.

use anyhow::Result;
use super::EntropyCoder;
use crate::container::EntropyMethod;

/// Simple range coder implementation
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
        if data.is_empty() {
            return Ok(Vec::new());
        }

        // Build frequency table
        let mut frequencies = [0u32; 256];
        for &byte in data {
            frequencies[byte as usize] += 1;
        }

        // Normalize frequencies to avoid overflow
        let total_freq = data.len() as u32;
        let mut normalized_freqs = [0u32; 256];
        let mut cumulative_freqs = [0u32; 257]; // 257 to include total

        for i in 0..256 {
            normalized_freqs[i] = (frequencies[i] * 65535) / total_freq;
            if normalized_freqs[i] == 0 && frequencies[i] > 0 {
                normalized_freqs[i] = 1; // Ensure non-zero for symbols that exist
            }
        }

        // Build cumulative frequency table
        for i in 0..256 {
            cumulative_freqs[i + 1] = cumulative_freqs[i] + normalized_freqs[i];
        }

        // Encode data
        let mut result = Vec::new();
        
        // Write frequency table
        for &freq in &normalized_freqs {
            result.extend_from_slice(&freq.to_le_bytes());
        }

        // Range coding
        let mut low = 0u64;
        let mut high = 0xFFFFFFFFu64;
        let _pending = 0u32;

        for &byte in data {
            let symbol = byte as usize;
            let range = high - low + 1;
            let symbol_low = cumulative_freqs[symbol] as u64;
            let symbol_high = cumulative_freqs[symbol + 1] as u64;
            let total_freq = cumulative_freqs[256] as u64;

            high = low + (range * symbol_high) / total_freq - 1;
            low = low + (range * symbol_low) / total_freq;

            // Normalize range
            while (low ^ high) < 0x1000000 {
                result.push((low >> 24) as u8);
                low <<= 8;
                high = (high << 8) | 0xFF;
            }
        }

        // Flush remaining bits
        result.push((low >> 24) as u8);
        result.push((low >> 16) as u8);
        result.push((low >> 8) as u8);
        result.push(low as u8);

        Ok(result)
    }

    fn decode(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        if data.len() < 256 * 4 {
            return Err(anyhow::anyhow!("Invalid range coded data"));
        }

        // Read frequency table
        let mut normalized_freqs = [0u32; 256];
        for i in 0..256 {
            let freq_bytes = &data[i * 4..(i + 1) * 4];
            normalized_freqs[i] = u32::from_le_bytes([freq_bytes[0], freq_bytes[1], freq_bytes[2], freq_bytes[3]]);
        }

        // Build cumulative frequency table
        let mut cumulative_freqs = [0u32; 257];
        for i in 0..256 {
            cumulative_freqs[i + 1] = cumulative_freqs[i] + normalized_freqs[i];
        }

        let total_freq = cumulative_freqs[256];
        if total_freq == 0 {
            return Err(anyhow::anyhow!("Invalid frequency table"));
        }

        // Read initial range
        let mut data_pos = 256 * 4;
        if data_pos + 4 > data.len() {
            return Err(anyhow::anyhow!("Insufficient data for range"));
        }

        let mut low = 0u64;
        let mut high = 0xFFFFFFFFu64;
        let mut value = ((data[data_pos] as u64) << 24) |
                       ((data[data_pos + 1] as u64) << 16) |
                       ((data[data_pos + 2] as u64) << 8) |
                       (data[data_pos + 3] as u64);
        data_pos += 4;

        let mut result = Vec::new();

        // Decode symbols
        loop {
            let range = high - low + 1;
            let scaled_value = ((value - low) * total_freq as u64) / range;
            
            // Find symbol
            let mut symbol = 0;
            for i in 0..256 {
                if scaled_value < cumulative_freqs[i + 1] as u64 {
                    symbol = i;
                    break;
                }
            }

            result.push(symbol as u8);

            // Update range
            let symbol_low = cumulative_freqs[symbol] as u64;
            let symbol_high = cumulative_freqs[symbol + 1] as u64;

            high = low + (range * symbol_high) / total_freq as u64 - 1;
            low = low + (range * symbol_low) / total_freq as u64;

            // Normalize range
            while (low ^ high) < 0x1000000 {
                if data_pos >= data.len() {
                    return Ok(result);
                }
                low = (low << 8) | (data[data_pos] as u64);
                high = (high << 8) | 0xFF;
                value = (value << 8) | (data[data_pos] as u64);
                data_pos += 1;
            }
        }
    }

    fn method(&self) -> EntropyMethod {
        EntropyMethod::RangeCoder
    }
}

impl Default for RangeCoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_coder_roundtrip() {
        let coder = RangeCoder::new();
        let original = b"Hello, world! This is a test of range coding.";
        
        let encoded = coder.encode(original).unwrap();
        let decoded = coder.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_range_coder_repeated_data() {
        let coder = RangeCoder::new();
        let original = b"AAAABBBBCCCCDDDD";
        
        let encoded = coder.encode(original).unwrap();
        let decoded = coder.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_range_coder_empty_data() {
        let coder = RangeCoder::new();
        let empty: &[u8] = &[];
        
        let encoded = coder.encode(empty).unwrap();
        let decoded = coder.decode(&encoded).unwrap();
        
        assert_eq!(empty, &decoded[..]);
    }

    #[test]
    fn test_range_coder_single_byte() {
        let coder = RangeCoder::new();
        let original = b"A";
        
        let encoded = coder.encode(original).unwrap();
        let decoded = coder.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_range_coder_high_entropy() {
        let coder = RangeCoder::new();
        let original: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        
        let encoded = coder.encode(&original).unwrap();
        let decoded = coder.decode(&encoded).unwrap();
        
        assert_eq!(original, decoded);
    }
}