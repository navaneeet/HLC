//! Utility functions for entropy coding
//! 
//! Provides helper functions for frequency analysis, bit manipulation,
//! and other entropy coding operations.


/// Calculate Shannon entropy of data
pub fn calculate_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let mut counts = [0u32; 256];
    for &byte in data {
        counts[byte as usize] += 1;
    }

    let mut entropy = 0.0;
    let data_len = data.len() as f64;
    
    for &count in counts.iter() {
        if count > 0 {
            let probability = count as f64 / data_len;
            entropy -= probability * probability.log2();
        }
    }

    entropy
}

/// Build frequency table from data
pub fn build_frequency_table(data: &[u8]) -> [u32; 256] {
    let mut frequencies = [0u32; 256];
    for &byte in data {
        frequencies[byte as usize] += 1;
    }
    frequencies
}

/// Normalize frequency table to avoid overflow
pub fn normalize_frequencies(frequencies: &[u32; 256], max_freq: u32) -> [u32; 256] {
    let total: u32 = frequencies.iter().sum();
    if total == 0 {
        return [0; 256];
    }

    let mut normalized = [0u32; 256];
    for i in 0..256 {
        normalized[i] = (frequencies[i] * max_freq) / total;
        if normalized[i] == 0 && frequencies[i] > 0 {
            normalized[i] = 1; // Ensure non-zero for existing symbols
        }
    }
    normalized
}

/// Build cumulative frequency table
pub fn build_cumulative_frequencies(frequencies: &[u32; 256]) -> [u32; 257] {
    let mut cumulative = [0u32; 257];
    for i in 0..256 {
        cumulative[i + 1] = cumulative[i] + frequencies[i];
    }
    cumulative
}

/// Calculate compression ratio
pub fn compression_ratio(original_size: usize, compressed_size: usize) -> f64 {
    if original_size == 0 {
        return 0.0;
    }
    compressed_size as f64 / original_size as f64
}

/// Calculate space savings percentage
pub fn space_savings(original_size: usize, compressed_size: usize) -> f64 {
    if original_size == 0 {
        return 0.0;
    }
    (1.0 - compression_ratio(original_size, compressed_size)) * 100.0
}

/// Estimate compression potential based on entropy
pub fn estimate_compression_potential(entropy: f64) -> f64 {
    // Theoretical maximum compression ratio based on entropy
    // This is a rough estimate and actual compression may vary
    entropy / 8.0
}

/// Check if data is highly compressible
pub fn is_highly_compressible(data: &[u8]) -> bool {
    let entropy = calculate_entropy(data);
    entropy < 4.0 // Low entropy suggests high compressibility
}

/// Check if data is already compressed
pub fn is_likely_compressed(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }

    let entropy = calculate_entropy(data);
    entropy > 7.5 // High entropy suggests already compressed data
}

/// Calculate byte frequency distribution
pub fn byte_frequency_distribution(data: &[u8]) -> Vec<(u8, u32)> {
    let frequencies = build_frequency_table(data);
    let mut distribution: Vec<(u8, u32)> = (0..256)
        .map(|i| (i as u8, frequencies[i]))
        .filter(|(_, count)| *count > 0)
        .collect();
    
    distribution.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by frequency descending
    distribution
}

/// Find most frequent bytes
pub fn most_frequent_bytes(data: &[u8], count: usize) -> Vec<(u8, u32)> {
    let mut distribution = byte_frequency_distribution(data);
    distribution.truncate(count);
    distribution
}

/// Calculate data redundancy
pub fn calculate_redundancy(data: &[u8]) -> f64 {
    let entropy = calculate_entropy(data);
    let max_entropy = 8.0; // Maximum entropy for 8-bit data
    max_entropy - entropy
}

/// Estimate optimal chunk size based on entropy
pub fn estimate_optimal_chunk_size(entropy: f64, base_size: usize) -> usize {
    if entropy < 2.0 {
        // Very low entropy - use larger chunks
        base_size * 2
    } else if entropy > 7.0 {
        // High entropy - use smaller chunks
        base_size / 2
    } else {
        base_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_calculation() {
        // Low entropy data
        let low_entropy = vec![0u8; 1000];
        assert!(calculate_entropy(&low_entropy) < 0.1);
        
        // High entropy data
        let high_entropy: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        assert!(calculate_entropy(&high_entropy) > 7.0);
    }

    #[test]
    fn test_frequency_table() {
        let data = b"Hello, world!";
        let frequencies = build_frequency_table(data);
        
        assert_eq!(frequencies[b'l' as usize], 3);
        assert_eq!(frequencies[b'o' as usize], 2);
        assert_eq!(frequencies[b'z' as usize], 0);
    }

    #[test]
    fn test_compression_ratio() {
        assert_eq!(compression_ratio(1000, 500), 0.5);
        assert_eq!(compression_ratio(1000, 1000), 1.0);
        assert_eq!(compression_ratio(1000, 2000), 2.0);
    }

    #[test]
    fn test_space_savings() {
        assert_eq!(space_savings(1000, 500), 50.0);
        assert_eq!(space_savings(1000, 1000), 0.0);
        assert_eq!(space_savings(1000, 2000), -100.0);
    }

    #[test]
    fn test_compressibility_detection() {
        let low_entropy = vec![0u8; 1000];
        assert!(is_highly_compressible(&low_entropy));
        
        let high_entropy: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        assert!(!is_highly_compressible(&high_entropy));
    }

    #[test]
    fn test_byte_distribution() {
        let data = b"Hello, world!";
        let distribution = byte_frequency_distribution(data);
        
        // Should be sorted by frequency
        for i in 1..distribution.len() {
            assert!(distribution[i-1].1 >= distribution[i].1);
        }
    }
}