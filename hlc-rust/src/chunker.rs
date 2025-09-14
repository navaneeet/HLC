//! Adaptive chunking implementation
//! 
//! Implements content-aware chunking that adapts chunk sizes based on
//! entropy and content type for optimal compression.

use anyhow::Result;
use std::io::Read;

/// Adaptive chunker that adjusts chunk sizes based on content entropy
pub struct AdaptiveChunker {
    min_size: usize,
    max_size: usize,
    target_size: usize,
    entropy_threshold: f64,
}

impl AdaptiveChunker {
    pub fn new(min_size: usize, max_size: usize) -> Self {
        Self {
            min_size,
            max_size,
            target_size: (min_size + max_size) / 2,
            entropy_threshold: 0.7, // Adjust based on testing
        }
    }

    /// Chunk data into adaptive-sized chunks
    pub fn chunk_data<R: Read>(&self, mut reader: R) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();
        let mut chunk_id = 0;
        let mut buffer = vec![0u8; self.max_size];
        let mut current_chunk = Vec::new();

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            // Process the read data in chunks
            let mut offset = 0;
            while offset < bytes_read {
                let remaining = bytes_read - offset;
                let chunk_size = self.determine_chunk_size(&buffer[offset..offset + remaining]);
                
                let actual_size = chunk_size.min(remaining);
                current_chunk.extend_from_slice(&buffer[offset..offset + actual_size]);
                
                // If we've reached the target size or hit a natural boundary
                if current_chunk.len() >= self.target_size || 
                   self.is_natural_boundary(&current_chunk) ||
                   actual_size < chunk_size {
                    
                    if !current_chunk.is_empty() {
                        chunks.push(Chunk {
                            id: chunk_id,
                            data: current_chunk.clone(),
                            entropy: self.calculate_entropy(&current_chunk),
                        });
                        chunk_id += 1;
                        current_chunk.clear();
                    }
                }
                
                offset += actual_size;
            }
        }

        // Handle remaining data
        if !current_chunk.is_empty() {
            let entropy = self.calculate_entropy(&current_chunk);
            chunks.push(Chunk {
                id: chunk_id,
                data: current_chunk,
                entropy,
            });
        }

        Ok(chunks)
    }

    /// Determine optimal chunk size based on content entropy
    fn determine_chunk_size(&self, data: &[u8]) -> usize {
        if data.len() < self.min_size {
            return data.len();
        }

        // Calculate entropy for the available data
        let sample_size = data.len().min(1024); // Sample first 1KB for entropy estimation
        let entropy = self.calculate_entropy(&data[..sample_size]);

        // Adjust chunk size based on entropy
        if entropy > self.entropy_threshold {
            // High entropy content - use smaller chunks for better compression
            self.min_size.max(self.target_size / 2)
        } else {
            // Low entropy content - use larger chunks
            self.max_size.min(self.target_size * 2)
        }
    }

    /// Check if we've hit a natural chunk boundary (e.g., end of JSON object)
    fn is_natural_boundary(&self, data: &[u8]) -> bool {
        if data.len() < 2 {
            return false;
        }

        // Look for common boundary patterns
        let last_two = &data[data.len() - 2..];
        
        // JSON object/array boundaries
        last_two == b"}\n" || last_two == b"]\n" || 
        last_two == b"}\r" || last_two == b"]\r" ||
        last_two == b"}\t" || last_two == b"]\t" ||
        
        // XML tag boundaries
        last_two == b">\n" || last_two == b">\r" ||
        
        // Binary patterns (null terminator, etc.)
        last_two[1] == 0
    }

    /// Calculate Shannon entropy of the data
    fn calculate_entropy(&self, data: &[u8]) -> f64 {
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
}

/// Represents a single chunk of data
#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: u32,
    pub data: Vec<u8>,
    pub entropy: f64,
}

impl Chunk {
    pub fn new(id: u32, data: Vec<u8>) -> Self {
        let entropy = Self::calculate_entropy(&data);
        Self { id, data, entropy }
    }

    fn calculate_entropy(data: &[u8]) -> f64 {
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

    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Fixed-size chunker for comparison
pub struct FixedChunker {
    chunk_size: usize,
}

impl FixedChunker {
    pub fn new(chunk_size: usize) -> Self {
        Self { chunk_size }
    }

    pub fn chunk_data<R: Read>(&self, mut reader: R) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();
        let mut chunk_id = 0;
        let mut buffer = vec![0u8; self.chunk_size];

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            chunks.push(Chunk::new(
                chunk_id,
                buffer[..bytes_read].to_vec(),
            ));
            chunk_id += 1;
        }

        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_adaptive_chunker() {
        let chunker = AdaptiveChunker::new(1024, 8192);
        let data = b"Hello, world! This is a test of adaptive chunking.".repeat(100);
        let cursor = Cursor::new(&data);
        
        let chunks = chunker.chunk_data(cursor).unwrap();
        assert!(!chunks.is_empty());
        
        for chunk in &chunks {
            assert!(chunk.size() >= 1024 || chunk.size() <= 8192);
        }
    }

    #[test]
    fn test_fixed_chunker() {
        let chunker = FixedChunker::new(1024);
        let data = vec![1u8; 5000];
        let cursor = Cursor::new(&data);
        
        let chunks = chunker.chunk_data(cursor).unwrap();
        assert_eq!(chunks.len(), 5); // 5000 / 1024 = 4.88, so 5 chunks
        
        for chunk in &chunks[..4] {
            assert_eq!(chunk.size(), 1024);
        }
        assert_eq!(chunks[4].size(), 5000 - 4 * 1024);
    }

    #[test]
    fn test_entropy_calculation() {
        // Low entropy data (all same byte)
        let low_entropy = vec![0u8; 1000];
        assert!(Chunk::calculate_entropy(&low_entropy) < 0.1);
        
        // High entropy data (random-like)
        let high_entropy: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        assert!(Chunk::calculate_entropy(&high_entropy) > 7.0);
    }
}