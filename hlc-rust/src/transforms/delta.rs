//! Delta encoding transform
//! 
//! Computes differences between consecutive bytes, which can improve
//! compression for data with local correlations.

use anyhow::Result;
use super::Transform;

/// Delta encoding transform
pub struct DeltaTransform {
    mode: DeltaMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeltaMode {
    Byte,    // Byte-wise delta
    Word,    // 16-bit word delta
    DWord,   // 32-bit dword delta
}

impl DeltaTransform {
    pub fn new() -> Self {
        Self {
            mode: DeltaMode::Byte,
        }
    }

    pub fn with_mode(mode: DeltaMode) -> Self {
        Self { mode }
    }
}

impl Transform for DeltaTransform {
    fn encode(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        match self.mode {
            DeltaMode::Byte => self.encode_byte_delta(data),
            DeltaMode::Word => self.encode_word_delta(data),
            DeltaMode::DWord => self.encode_dword_delta(data),
        }
    }

    fn decode(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        match self.mode {
            DeltaMode::Byte => self.decode_byte_delta(data),
            DeltaMode::Word => self.decode_word_delta(data),
            DeltaMode::DWord => self.decode_dword_delta(data),
        }
    }

    fn id(&self) -> u8 {
        1 // Delta transform ID
    }
}

impl DeltaTransform {
    fn encode_byte_delta(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut result = Vec::with_capacity(data.len());
        result.push(data[0]); // First byte is stored as-is
        
        for i in 1..data.len() {
            let delta = data[i].wrapping_sub(data[i - 1]);
            result.push(delta);
        }
        
        Ok(result)
    }

    fn decode_byte_delta(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut result = Vec::with_capacity(data.len());
        result.push(data[0]); // First byte is stored as-is
        
        for i in 1..data.len() {
            let original = data[i].wrapping_add(result[i - 1]);
            result.push(original);
        }
        
        Ok(result)
    }

    fn encode_word_delta(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 2 {
            return Ok(data.to_vec());
        }

        let mut result = Vec::with_capacity(data.len());
        result.extend_from_slice(&data[..2]); // First word is stored as-is
        
        for i in (2..data.len()).step_by(2) {
            if i + 1 < data.len() {
                let current = u16::from_le_bytes([data[i], data[i + 1]]);
                let previous = u16::from_le_bytes([data[i - 2], data[i - 1]]);
                let delta = current.wrapping_sub(previous);
                result.extend_from_slice(&delta.to_le_bytes());
            } else {
                // Handle odd number of bytes
                result.push(data[i]);
            }
        }
        
        Ok(result)
    }

    fn decode_word_delta(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 2 {
            return Ok(data.to_vec());
        }

        let mut result = Vec::with_capacity(data.len());
        result.extend_from_slice(&data[..2]); // First word is stored as-is
        
        for i in (2..data.len()).step_by(2) {
            if i + 1 < data.len() {
                let delta = u16::from_le_bytes([data[i], data[i + 1]]);
                let previous = u16::from_le_bytes([result[i - 2], result[i - 1]]);
                let current = delta.wrapping_add(previous);
                result.extend_from_slice(&current.to_le_bytes());
            } else {
                // Handle odd number of bytes
                result.push(data[i]);
            }
        }
        
        Ok(result)
    }

    fn encode_dword_delta(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 4 {
            return Ok(data.to_vec());
        }

        let mut result = Vec::with_capacity(data.len());
        result.extend_from_slice(&data[..4]); // First dword is stored as-is
        
        for i in (4..data.len()).step_by(4) {
            if i + 3 < data.len() {
                let current = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
                let previous = u32::from_le_bytes([data[i - 4], data[i - 3], data[i - 2], data[i - 1]]);
                let delta = current.wrapping_sub(previous);
                result.extend_from_slice(&delta.to_le_bytes());
            } else {
                // Handle remaining bytes
                result.extend_from_slice(&data[i..]);
                break;
            }
        }
        
        Ok(result)
    }

    fn decode_dword_delta(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 4 {
            return Ok(data.to_vec());
        }

        let mut result = Vec::with_capacity(data.len());
        result.extend_from_slice(&data[..4]); // First dword is stored as-is
        
        for i in (4..data.len()).step_by(4) {
            if i + 3 < data.len() {
                let delta = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
                let previous = u32::from_le_bytes([result[i - 4], result[i - 3], result[i - 2], result[i - 1]]);
                let current = delta.wrapping_add(previous);
                result.extend_from_slice(&current.to_le_bytes());
            } else {
                // Handle remaining bytes
                result.extend_from_slice(&data[i..]);
                break;
            }
        }
        
        Ok(result)
    }
}

impl Default for DeltaTransform {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_delta_roundtrip() {
        let transform = DeltaTransform::new();
        let original = b"Hello, world!";
        
        let encoded = transform.encode(original).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_word_delta_roundtrip() {
        let transform = DeltaTransform::with_mode(DeltaMode::Word);
        let original = b"Hello, world! This is a test.";
        
        let encoded = transform.encode(original).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_dword_delta_roundtrip() {
        let transform = DeltaTransform::with_mode(DeltaMode::DWord);
        let original = b"Hello, world! This is a test for dword delta.";
        
        let encoded = transform.encode(original).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_empty_data() {
        let transform = DeltaTransform::new();
        let empty: &[u8] = &[];
        
        let encoded = transform.encode(empty).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(empty, &decoded[..]);
    }

    #[test]
    fn test_single_byte() {
        let transform = DeltaTransform::new();
        let single = b"A";
        
        let encoded = transform.encode(single).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(single, &decoded[..]);
    }
}