/// Run-Length Encoding implementation optimized for zero sequences
/// Format: For zero runs: [0x00][COUNT], for non-zero bytes: [BYTE]
/// This is particularly effective for sparse data with many zero sequences

pub fn encode(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }

    let mut encoded = Vec::with_capacity(data.len());
    let mut i = 0;
    
    while i < data.len() {
        if data[i] == 0 {
            // Count consecutive zeros
            let mut count = 0;
            while i + count < data.len() && data[i + count] == 0 && count < 255 {
                count += 1;
            }
            
            // Encode zero run: 0x00 followed by count
            encoded.push(0);
            encoded.push(count as u8);
            i += count;
        } else {
            // Non-zero byte, store as-is
            encoded.push(data[i]);
            i += 1;
        }
    }
    
    encoded
}

pub fn decode(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }

    let mut decoded = Vec::with_capacity(data.len() * 2);
    let mut i = 0;
    
    while i < data.len() {
        if data[i] == 0 {
            // This is a zero run marker
            if i + 1 < data.len() {
                let count = data[i + 1] as usize;
                // Add 'count' zeros
                for _ in 0..count {
                    decoded.push(0);
                }
                i += 2;
            } else {
                // Malformed data - treat as single zero
                decoded.push(0);
                i += 1;
            }
        } else {
            // Regular non-zero byte
            decoded.push(data[i]);
            i += 1;
        }
    }
    
    decoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rle_empty() {
        let data = vec![];
        let encoded = encode(&data);
        let decoded = decode(&encoded);
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_rle_no_zeros() {
        let data = vec![1, 2, 3, 4, 5];
        let encoded = encode(&data);
        let decoded = decode(&encoded);
        assert_eq!(data, decoded);
        assert_eq!(encoded, data); // Should be unchanged
    }

    #[test]
    fn test_rle_with_zeros() {
        let data = vec![1, 0, 0, 0, 2, 0, 3];
        let encoded = encode(&data);
        let decoded = decode(&encoded);
        assert_eq!(data, decoded);
        
        // Should be: [1, 0, 3, 2, 0, 1, 3]
        // 1 -> 1, three zeros -> 0,3, 2 -> 2, one zero -> 0,1, 3 -> 3
        let expected = vec![1, 0, 3, 2, 0, 1, 3];
        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_rle_long_zero_run() {
        let mut data = vec![1];
        data.extend(vec![0; 100]);
        data.push(2);
        
        let encoded = encode(&data);
        let decoded = decode(&encoded);
        assert_eq!(data, decoded);
        
        // Should compress significantly
        assert!(encoded.len() < data.len());
    }
}