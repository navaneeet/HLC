//! Dictionary-based compression transform
//! 
//! Uses LZ77-style dictionary compression to find and replace repeated
//! sequences with references to previous occurrences.

use anyhow::Result;
use super::Transform;

/// Dictionary compression transform using LZ77-style algorithm
pub struct DictionaryTransform {
    window_size: usize,
    min_match_length: usize,
    max_match_length: usize,
}

impl DictionaryTransform {
    pub fn new() -> Self {
        Self {
            window_size: 32 * 1024, // 32KB window
            min_match_length: 3,
            max_match_length: 258, // Standard LZ77 max
        }
    }

    pub fn with_params(window_size: usize, min_match_length: usize, max_match_length: usize) -> Self {
        Self {
            window_size,
            min_match_length,
            max_match_length,
        }
    }
}

impl Transform for DictionaryTransform {
    fn encode(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::new();
        let mut i = 0;

        while i < data.len() {
            let (match_length, match_distance) = self.find_longest_match(data, i);
            
            if match_length >= self.min_match_length {
                // Encode as match: [0xFF, distance_low, distance_high, length]
                result.push(0xFF);
                result.push((match_distance & 0xFF) as u8);
                result.push(((match_distance >> 8) & 0xFF) as u8);
                result.push(match_length as u8);
                i += match_length;
            } else {
                // Encode as literal
                if data[i] == 0xFF {
                    // Escape literal 0xFF as [0xFF, 0x00, 0x00, 0x01]
                    result.push(0xFF);
                    result.push(0x00);
                    result.push(0x00);
                    result.push(0x01);
                } else {
                    result.push(data[i]);
                }
                i += 1;
            }
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
                let distance = (data[i + 1] as usize) | ((data[i + 2] as usize) << 8);
                let length = data[i + 3] as usize;

                if distance == 0 && length == 1 {
                    // Escaped literal 0xFF
                    result.push(0xFF);
                } else {
                    // Copy match from previous data
                    let start_pos = result.len().saturating_sub(distance);
                    for j in 0..length {
                        if start_pos + j < result.len() {
                            result.push(result[start_pos + j]);
                        }
                    }
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
        4 // Dictionary transform ID
    }
}

impl DictionaryTransform {
    /// Find the longest match in the sliding window
    fn find_longest_match(&self, data: &[u8], pos: usize) -> (usize, usize) {
        let mut best_length = 0;
        let mut best_distance = 0;

        let search_start = pos.saturating_sub(self.window_size);
        let max_length = (data.len() - pos).min(self.max_match_length);

        for search_pos in search_start..pos {
            let mut match_length = 0;
            
            // Count matching bytes
            while match_length < max_length
                && pos + match_length < data.len()
                && search_pos + match_length < pos
                && data[search_pos + match_length] == data[pos + match_length] {
                match_length += 1;
            }

            if match_length > best_length {
                best_length = match_length;
                best_distance = pos - search_pos;
            }
        }

        (best_length, best_distance)
    }
}

impl Default for DictionaryTransform {
    fn default() -> Self {
        Self::new()
    }
}

/// LZ4-style dictionary transform for better performance
pub struct LZ4DictionaryTransform {
    max_distance: usize,
    max_length: usize,
}

impl LZ4DictionaryTransform {
    pub fn new() -> Self {
        Self {
            max_distance: 65535, // 64KB
            max_length: 65535,    // 64KB
        }
    }
}

impl Transform for LZ4DictionaryTransform {
    fn encode(&self, data: &[u8]) -> Result<Vec<u8>> {
        // For now, delegate to the standard dictionary transform
        // In a full implementation, this would use LZ4's specific encoding
        let standard_transform = DictionaryTransform::with_params(
            self.max_distance,
            4, // LZ4 minimum match length
            self.max_length.min(258),
        );
        standard_transform.encode(data)
    }

    fn decode(&self, data: &[u8]) -> Result<Vec<u8>> {
        // For now, delegate to the standard dictionary transform
        let standard_transform = DictionaryTransform::with_params(
            self.max_distance,
            4,
            self.max_length.min(258),
        );
        standard_transform.decode(data)
    }

    fn id(&self) -> u8 {
        5 // LZ4 dictionary transform ID
    }
}

impl Default for LZ4DictionaryTransform {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dictionary_roundtrip() {
        let transform = DictionaryTransform::new();
        let original = b"Hello, world! Hello, world!";
        
        let encoded = transform.encode(original).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_dictionary_repeated_patterns() {
        let transform = DictionaryTransform::new();
        let original = b"ABCABCABCABCABC";
        
        let encoded = transform.encode(original).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_dictionary_no_matches() {
        let transform = DictionaryTransform::new();
        let original = b"ABCDEFGHIJKLMNOP";
        
        let encoded = transform.encode(original).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_dictionary_escape_sequence() {
        let transform = DictionaryTransform::new();
        let original = b"Hello\xFFWorld";
        
        let encoded = transform.encode(original).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_empty_data() {
        let transform = DictionaryTransform::new();
        let empty: &[u8] = &[];
        
        let encoded = transform.encode(empty).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(empty, &decoded[..]);
    }

    #[test]
    fn test_lz4_dictionary() {
        let transform = LZ4DictionaryTransform::new();
        let original = b"Hello, world! Hello, world! This is a test.";
        
        let encoded = transform.encode(original).unwrap();
        let decoded = transform.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }
}