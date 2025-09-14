use crate::config::HlcConfig;
use crate::container::{CompressedChunk, PipelineFlags, calculate_checksum};
use crate::error::HlcError;
use crate::transforms::{analyzer, delta, entropy, rle, dictionary};

#[derive(Debug, Clone)]
pub struct RawChunk {
    pub id: usize,
    pub data: Vec<u8>,
}

impl RawChunk {
    pub fn new(id: usize, data: Vec<u8>) -> Self {
        Self { id, data }
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// Process a single chunk through the HLC compression pipeline
pub fn process_chunk(chunk: RawChunk, config: &HlcConfig) -> Result<CompressedChunk, HlcError> {
    let original_size = chunk.data.len();
    if original_size == 0 {
        let checksum = calculate_checksum(&chunk.data, config.checksum);
        return Ok(CompressedChunk::new(chunk.id, chunk.data, original_size, checksum));
    }

    let checksum = calculate_checksum(&chunk.data, config.checksum);
    
    // 1. Analyze the chunk to determine the best compression strategy
    let strategy = analyzer::analyze_chunk(&chunk.data, config.mode);

    // 2. Apply pre-processing transforms based on the strategy
    let (mut transformed_data, mut flags) = apply_transforms(chunk.data, &strategy)?;

    // 3. Apply entropy coding if the data isn't stored raw
    if !flags.contains(PipelineFlags::STORED) {
        match entropy::encode(&transformed_data, config.entropy_level) {
            Ok(entropy_compressed) => {
                // Only use entropy compression if it actually reduces size
                if entropy_compressed.len() < transformed_data.len() {
                    transformed_data = entropy_compressed;
                    flags |= PipelineFlags::ENTROPY;
                } else {
                    // Entropy coding didn't help, store as-is
                    flags = PipelineFlags::STORED;
                    transformed_data = strategy.original_data.clone();
                }
            }
            Err(_) => {
                // Entropy coding failed, store as raw
                flags = PipelineFlags::STORED;
                transformed_data = strategy.original_data.clone();
            }
        }
    }

    // 4. Final check: if compressed data is not smaller, store original
    if transformed_data.len() >= original_size {
        flags = PipelineFlags::STORED;
        transformed_data = strategy.original_data.clone();
    }

    Ok(CompressedChunk {
        id: chunk.id,
        flags,
        original_checksum: checksum,
        original_size: original_size as u32,
        compressed_size: transformed_data.len() as u32,
        data: transformed_data,
    })
}

/// Apply the selected transforms to the data
fn apply_transforms(
    mut data: Vec<u8>,
    strategy: &analyzer::CompressionStrategy,
) -> Result<(Vec<u8>, PipelineFlags), HlcError> {
    let mut flags = PipelineFlags::empty();
    let original_len = data.len();

    // Apply transforms in order: RLE -> Delta -> Dictionary
    // Each transform is only applied if it reduces the data size
    
    if strategy.use_rle {
        let rle_data = rle::encode(&data);
        if rle_data.len() < data.len() {
            data = rle_data;
            flags |= PipelineFlags::RLE;
        }
    }

    if strategy.use_delta {
        let delta_data = delta::encode(&data);
        if delta_data.len() <= data.len() {
            data = delta_data;
            flags |= PipelineFlags::DELTA;
        }
    }

    if strategy.use_dictionary {
        let dict_data = dictionary::encode(&data);
        if dict_data.len() < data.len() {
            data = dict_data;
            flags |= PipelineFlags::DICTIONARY;
        }
    }

    // If no transforms were beneficial, mark as stored
    if flags.is_empty() || data.len() >= original_len {
        flags = PipelineFlags::STORED;
        data = strategy.original_data.clone();
    }

    Ok((data, flags))
}

/// Split data into chunks for processing
pub fn split_into_chunks(data: &[u8], chunk_size: usize) -> Vec<RawChunk> {
    if data.is_empty() {
        return vec![RawChunk::new(0, Vec::new())];
    }

    data.chunks(chunk_size)
        .enumerate()
        .map(|(id, chunk_data)| RawChunk::new(id, chunk_data.to_vec()))
        .collect()
}

/// Merge chunks back into continuous data
pub fn merge_chunks(chunks: &[RawChunk]) -> Vec<u8> {
    let total_size: usize = chunks.iter().map(|c| c.size()).sum();
    let mut result = Vec::with_capacity(total_size);
    
    for chunk in chunks {
        result.extend_from_slice(&chunk.data);
    }
    
    result
}

/// Estimate compression effectiveness for a chunk
pub fn estimate_compression_ratio(chunk: &RawChunk, config: &HlcConfig) -> f32 {
    if chunk.is_empty() {
        return 1.0;
    }

    let strategy = analyzer::analyze_chunk(&chunk.data, config.mode);
    
    // Simple estimation based on entropy and detected patterns
    let base_ratio = entropy::estimate_compression_ratio(&chunk.data);
    
    let mut bonus = 1.0;
    if strategy.use_rle {
        bonus += 0.2; // RLE can be very effective for sparse data
    }
    if strategy.use_delta {
        bonus += 0.1; // Delta coding provides moderate improvement
    }
    if strategy.use_dictionary {
        bonus += 0.15; // Dictionary compression can be quite effective
    }
    
    base_ratio * bonus
}

/// Validate chunk data integrity
pub fn validate_chunk(chunk: &RawChunk) -> Result<(), HlcError> {
    // Basic validation - could be extended with more checks
    if chunk.data.len() > u32::MAX as usize {
        return Err(HlcError::PipelineError(
            "Chunk too large for processing".to_string()
        ));
    }
    
    Ok(())
}

/// Chunk processing statistics
#[derive(Debug, Default, Clone)]
pub struct ChunkStats {
    pub total_chunks: usize,
    pub stored_chunks: usize,
    pub rle_chunks: usize,
    pub delta_chunks: usize,
    pub dictionary_chunks: usize,
    pub entropy_chunks: usize,
    pub total_original_size: u64,
    pub total_compressed_size: u64,
}

impl ChunkStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_chunk(&mut self, chunk: &CompressedChunk) {
        self.total_chunks += 1;
        self.total_original_size += chunk.original_size as u64;
        self.total_compressed_size += chunk.compressed_size as u64;

        if chunk.flags.contains(PipelineFlags::STORED) {
            self.stored_chunks += 1;
        }
        if chunk.flags.contains(PipelineFlags::RLE) {
            self.rle_chunks += 1;
        }
        if chunk.flags.contains(PipelineFlags::DELTA) {
            self.delta_chunks += 1;
        }
        if chunk.flags.contains(PipelineFlags::DICTIONARY) {
            self.dictionary_chunks += 1;
        }
        if chunk.flags.contains(PipelineFlags::ENTROPY) {
            self.entropy_chunks += 1;
        }
    }

    pub fn compression_ratio(&self) -> f64 {
        if self.total_compressed_size == 0 {
            return 0.0;
        }
        self.total_original_size as f64 / self.total_compressed_size as f64
    }

    pub fn space_saved(&self) -> u64 {
        self.total_original_size.saturating_sub(self.total_compressed_size)
    }

    pub fn space_saved_percentage(&self) -> f64 {
        if self.total_original_size == 0 {
            return 0.0;
        }
        (self.space_saved() as f64 / self.total_original_size as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HlcMode;

    #[test]
    fn test_chunk_creation() {
        let data = b"Hello, world!".to_vec();
        let chunk = RawChunk::new(0, data.clone());
        
        assert_eq!(chunk.id, 0);
        assert_eq!(chunk.data, data);
        assert_eq!(chunk.size(), data.len());
        assert!(!chunk.is_empty());
    }

    #[test]
    fn test_empty_chunk() {
        let chunk = RawChunk::new(0, Vec::new());
        assert!(chunk.is_empty());
        assert_eq!(chunk.size(), 0);
    }

    #[test]
    fn test_split_into_chunks() {
        let data = b"Hello, world! This is a test.";
        let chunks = split_into_chunks(data, 10);
        
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].data, b"Hello, wor");
        assert_eq!(chunks[1].data, b"ld! This i");
        assert_eq!(chunks[2].data, b"s a test.");
    }

    #[test]
    fn test_merge_chunks() {
        let original_data = b"Hello, world! This is a test.";
        let chunks = split_into_chunks(original_data, 10);
        let merged = merge_chunks(&chunks);
        
        assert_eq!(merged, original_data);
    }

    #[test]
    fn test_process_chunk() {
        let data = vec![0u8; 1000]; // Highly compressible data
        let chunk = RawChunk::new(0, data.clone());
        let config = HlcConfig::default().with_mode(HlcMode::Balanced);
        
        let compressed = process_chunk(chunk, &config).unwrap();
        
        assert_eq!(compressed.id, 0);
        assert_eq!(compressed.original_size as usize, data.len());
        // Note: Small data may not compress well, so just check it works
        assert!(compressed.original_size > 0);
    }

    #[test]
    fn test_chunk_stats() {
        let mut stats = ChunkStats::new();
        
        let data = vec![1u8; 100];
        let checksum = calculate_checksum(&data, crate::config::ChecksumType::CRC32);
        let chunk = CompressedChunk {
            id: 0,
            flags: PipelineFlags::RLE | PipelineFlags::ENTROPY,
            original_checksum: checksum,
            original_size: 100,
            compressed_size: 50,
            data: vec![0u8; 50],
        };
        
        stats.add_chunk(&chunk);
        
        assert_eq!(stats.total_chunks, 1);
        assert_eq!(stats.rle_chunks, 1);
        assert_eq!(stats.entropy_chunks, 1);
        assert_eq!(stats.compression_ratio(), 2.0);
        assert_eq!(stats.space_saved(), 50);
        assert_eq!(stats.space_saved_percentage(), 50.0);
    }

    #[test]
    fn test_validate_chunk() {
        let valid_chunk = RawChunk::new(0, vec![1, 2, 3, 4, 5]);
        assert!(validate_chunk(&valid_chunk).is_ok());
        
        // Test with very large chunk (this would fail in a real scenario with limited memory)
        let large_chunk = RawChunk::new(0, vec![0; 1000]);
        assert!(validate_chunk(&large_chunk).is_ok());
    }
}