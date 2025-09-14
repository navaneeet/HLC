/// Delta coding implementation
/// Transforms data[i] = data[i] - data[i-1] for i > 0
/// This is effective for data with gradual changes or sequential patterns

pub fn encode(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }
    
    let mut encoded = Vec::with_capacity(data.len());
    
    // First byte is stored as-is (reference point)
    encoded.push(data[0]);
    
    // Subsequent bytes are stored as differences from previous byte
    for i in 1..data.len() {
        let delta = data[i].wrapping_sub(data[i - 1]);
        encoded.push(delta);
    }
    
    encoded
}

pub fn decode(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }
    
    let mut decoded = Vec::with_capacity(data.len());
    
    // First byte is the reference point
    decoded.push(data[0]);
    
    // Reconstruct subsequent bytes by adding deltas
    for i in 1..data.len() {
        let previous = decoded[i - 1];
        let current = data[i].wrapping_add(previous);
        decoded.push(current);
    }
    
    decoded
}

/// Advanced delta encoding that can handle 16-bit and 32-bit integers
/// stored in little-endian format within byte arrays
pub fn encode_advanced(data: &[u8], word_size: usize) -> Vec<u8> {
    match word_size {
        1 => encode(data),
        2 => encode_u16_delta(data),
        4 => encode_u32_delta(data),
        _ => encode(data), // Fallback to byte-wise delta
    }
}

pub fn decode_advanced(data: &[u8], word_size: usize) -> Vec<u8> {
    match word_size {
        1 => decode(data),
        2 => decode_u16_delta(data),
        4 => decode_u32_delta(data),
        _ => decode(data), // Fallback to byte-wise delta
    }
}

fn encode_u16_delta(data: &[u8]) -> Vec<u8> {
    if data.len() < 2 {
        return data.to_vec();
    }
    
    let mut result = Vec::with_capacity(data.len());
    let words: Vec<u16> = data.chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    
    if words.is_empty() {
        return data.to_vec();
    }
    
    // Store first word as-is
    result.extend_from_slice(&words[0].to_le_bytes());
    
    // Store deltas
    for i in 1..words.len() {
        let delta = words[i].wrapping_sub(words[i - 1]);
        result.extend_from_slice(&delta.to_le_bytes());
    }
    
    // Handle remaining bytes if data length is not multiple of 2
    if data.len() % 2 == 1 {
        result.push(data[data.len() - 1]);
    }
    
    result
}

fn decode_u16_delta(data: &[u8]) -> Vec<u8> {
    if data.len() < 2 {
        return data.to_vec();
    }
    
    let mut result = Vec::with_capacity(data.len());
    let word_count = data.len() / 2;
    
    if word_count == 0 {
        return data.to_vec();
    }
    
    // Reconstruct first word
    let mut current_word = u16::from_le_bytes([data[0], data[1]]);
    result.extend_from_slice(&current_word.to_le_bytes());
    
    // Reconstruct subsequent words
    for i in 1..word_count {
        let delta = u16::from_le_bytes([data[i * 2], data[i * 2 + 1]]);
        current_word = current_word.wrapping_add(delta);
        result.extend_from_slice(&current_word.to_le_bytes());
    }
    
    // Handle remaining byte if data length is not multiple of 2
    if data.len() % 2 == 1 {
        result.push(data[data.len() - 1]);
    }
    
    result
}

fn encode_u32_delta(data: &[u8]) -> Vec<u8> {
    if data.len() < 4 {
        return data.to_vec();
    }
    
    let mut result = Vec::with_capacity(data.len());
    let words: Vec<u32> = data.chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();
    
    if words.is_empty() {
        return data.to_vec();
    }
    
    // Store first word as-is
    result.extend_from_slice(&words[0].to_le_bytes());
    
    // Store deltas
    for i in 1..words.len() {
        let delta = words[i].wrapping_sub(words[i - 1]);
        result.extend_from_slice(&delta.to_le_bytes());
    }
    
    // Handle remaining bytes
    let remainder = data.len() % 4;
    if remainder > 0 {
        let start = data.len() - remainder;
        result.extend_from_slice(&data[start..]);
    }
    
    result
}

fn decode_u32_delta(data: &[u8]) -> Vec<u8> {
    if data.len() < 4 {
        return data.to_vec();
    }
    
    let mut result = Vec::with_capacity(data.len());
    let word_count = data.len() / 4;
    
    if word_count == 0 {
        return data.to_vec();
    }
    
    // Reconstruct first word
    let mut current_word = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    result.extend_from_slice(&current_word.to_le_bytes());
    
    // Reconstruct subsequent words
    for i in 1..word_count {
        let base = i * 4;
        let delta = u32::from_le_bytes([data[base], data[base + 1], data[base + 2], data[base + 3]]);
        current_word = current_word.wrapping_add(delta);
        result.extend_from_slice(&current_word.to_le_bytes());
    }
    
    // Handle remaining bytes
    let remainder = data.len() % 4;
    if remainder > 0 {
        let start = data.len() - remainder;
        result.extend_from_slice(&data[start..]);
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_empty() {
        let data = vec![];
        let encoded = encode(&data);
        let decoded = decode(&encoded);
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_delta_single() {
        let data = vec![42];
        let encoded = encode(&data);
        let decoded = decode(&encoded);
        assert_eq!(data, decoded);
        assert_eq!(encoded, vec![42]);
    }

    #[test]
    fn test_delta_sequential() {
        let data = vec![10, 11, 12, 13, 14];
        let encoded = encode(&data);
        let decoded = decode(&encoded);
        assert_eq!(data, decoded);
        
        // Deltas should be [10, 1, 1, 1, 1]
        let expected = vec![10, 1, 1, 1, 1];
        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_delta_with_wraparound() {
        let data = vec![255, 1, 3];
        let encoded = encode(&data);
        let decoded = decode(&encoded);
        assert_eq!(data, decoded);
        
        // 1 - 255 = 2 (with wrapping), 3 - 1 = 2
        let expected = vec![255, 2, 2];
        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_u16_delta() {
        let data = vec![0x10, 0x00, 0x11, 0x00, 0x12, 0x00]; // 16, 17, 18 in little-endian
        let encoded = encode_u16_delta(&data);
        let decoded = decode_u16_delta(&encoded);
        assert_eq!(data, decoded);
    }
}