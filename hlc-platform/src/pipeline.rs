use crate::chunk::{process_chunk, split_into_chunks, merge_chunks, ChunkStats, RawChunk};
use crate::config::HlcConfig;
use crate::container::{write_hlc_container, read_hlc_container, CompressedChunk};
use crate::error::HlcError;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug)]
pub struct CompressionStats {
    pub original_size: u64,
    pub compressed_size: u64,
    pub ratio: f64,
    pub chunks_processed: usize,
    pub processing_time: std::time::Duration,
    pub chunk_stats: ChunkStats,
}

impl CompressionStats {
    pub fn new() -> Self {
        Self {
            original_size: 0,
            compressed_size: 0,
            ratio: 0.0,
            chunks_processed: 0,
            processing_time: std::time::Duration::from_secs(0),
            chunk_stats: ChunkStats::new(),
        }
    }

    pub fn space_saved(&self) -> u64 {
        self.original_size.saturating_sub(self.compressed_size)
    }

    pub fn space_saved_percentage(&self) -> f64 {
        if self.original_size == 0 {
            return 0.0;
        }
        (self.space_saved() as f64 / self.original_size as f64) * 100.0
    }

    pub fn throughput_mbps(&self) -> f64 {
        if self.processing_time.as_secs_f64() == 0.0 {
            return 0.0;
        }
        (self.original_size as f64 / (1024.0 * 1024.0)) / self.processing_time.as_secs_f64()
    }
}

/// Main compression function using the HLC pipeline
pub fn compress<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    config: &HlcConfig,
) -> Result<CompressionStats, HlcError> {
    let start_time = Instant::now();
    
    // Read all input data
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    let original_size = buffer.len() as u64;

    if original_size == 0 {
        // Handle empty input
        let chunks = vec![];
        write_hlc_container(writer, &chunks, config)?;
        return Ok(CompressionStats {
            original_size: 0,
            compressed_size: 0,
            ratio: 1.0,
            chunks_processed: 0,
            processing_time: start_time.elapsed(),
            chunk_stats: ChunkStats::new(),
        });
    }

    // Set up progress bar
    let pb = ProgressBar::new(original_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}) {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    pb.set_message("Compressing...");

    // Split data into chunks
    let raw_chunks = split_into_chunks(&buffer, config.chunk_size);
    let total_chunks = raw_chunks.len();

    // Configure the global thread pool for rayon
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.threads)
        .build()
        .map_err(|e| HlcError::ThreadPoolError(e.to_string()))?;

    // Shared statistics collector
    let stats = Arc::new(Mutex::new(ChunkStats::new()));
    let pb_clone = pb.clone();

    // Process chunks in parallel
    let compressed_chunks: Result<Vec<CompressedChunk>, HlcError> = pool.install(|| {
        raw_chunks
            .into_par_iter()
            .map(|chunk| {
                let chunk_size = chunk.size() as u64;
                let result = process_chunk(chunk, config);
                
                // Update progress and stats
                if let Ok(ref compressed_chunk) = result {
                    if let Ok(mut stats_guard) = stats.lock() {
                        stats_guard.add_chunk(compressed_chunk);
                    }
                }
                
                pb_clone.inc(chunk_size);
                result
            })
            .collect()
    });

    let mut compressed_chunks = compressed_chunks?;
    pb.finish_with_message("Compression complete");

    // Ensure chunks are in the correct order
    compressed_chunks.sort_by_key(|c| c.id);

    // Write the container format to the output stream
    let compressed_size = write_hlc_container(writer, &compressed_chunks, config)?;

    let processing_time = start_time.elapsed();
    let ratio = if compressed_size > 0 {
        original_size as f64 / compressed_size as f64
    } else {
        0.0
    };

    let final_stats = stats.lock().unwrap().clone();

    Ok(CompressionStats {
        original_size,
        compressed_size,
        ratio,
        chunks_processed: total_chunks,
        processing_time,
        chunk_stats: final_stats,
    })
}

/// Main decompression function
pub fn decompress<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    num_threads: usize,
) -> Result<(), HlcError> {
    let start_time = Instant::now();
    
    // Read the container
    let (compressed_chunks, config) = read_hlc_container(reader)?;
    let total_chunks = compressed_chunks.len();

    if total_chunks == 0 {
        return Ok(()); // Empty file
    }

    // Set up progress bar
    let pb = ProgressBar::new(total_chunks as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] Chunks {pos}/{len} ({per_sec}) {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    pb.set_message("Decompressing...");

    // Configure thread pool
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .map_err(|e| HlcError::ThreadPoolError(e.to_string()))?;

    let pb_clone = pb.clone();

    // Decompress chunks in parallel
    let decompressed_chunks: Result<Vec<RawChunk>, HlcError> = pool.install(|| {
        compressed_chunks
            .into_par_iter()
            .map(|chunk| {
                let result = chunk.decompress(&config);
                pb_clone.inc(1);
                result
            })
            .collect()
    });

    let mut decompressed_chunks = decompressed_chunks?;
    pb.finish_with_message("Decompression complete");

    // Ensure chunks are in the correct order
    decompressed_chunks.sort_by_key(|c| c.id);

    // Merge chunks and write to output
    let merged_data = merge_chunks(&decompressed_chunks);
    writer.write_all(&merged_data)?;

    println!("Decompression completed in {:?}", start_time.elapsed());
    Ok(())
}

/// Validate a compressed file without fully decompressing it
pub fn validate<R: Read>(reader: &mut R) -> Result<bool, HlcError> {
    let (compressed_chunks, config) = read_hlc_container(reader)?;
    
    println!("Validating {} chunks...", compressed_chunks.len());
    
    let pb = ProgressBar::new(compressed_chunks.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] Validating {pos}/{len} chunks")
            .unwrap()
            .progress_chars("#>-")
    );

    // Validate each chunk can be decompressed correctly
    for chunk in compressed_chunks {
        chunk.decompress(&config)?;
        pb.inc(1);
    }

    pb.finish_with_message("Validation complete");
    Ok(true)
}

/// Get information about a compressed file
pub fn info<R: Read>(reader: &mut R) -> Result<FileInfo, HlcError> {
    let (compressed_chunks, config) = read_hlc_container(reader)?;
    
    let mut info = FileInfo {
        version: 1,
        checksum_type: config.checksum,
        total_chunks: compressed_chunks.len(),
        original_size: 0,
        compressed_size: 0,
        compression_ratio: 0.0,
        chunk_stats: ChunkStats::new(),
    };

    for chunk in &compressed_chunks {
        info.original_size += chunk.original_size as u64;
        info.compressed_size += chunk.compressed_size as u64;
        info.chunk_stats.add_chunk(chunk);
    }

    info.compression_ratio = if info.compressed_size > 0 {
        info.original_size as f64 / info.compressed_size as f64
    } else {
        0.0
    };

    Ok(info)
}

#[derive(Debug)]
pub struct FileInfo {
    pub version: u8,
    pub checksum_type: crate::config::ChecksumType,
    pub total_chunks: usize,
    pub original_size: u64,
    pub compressed_size: u64,
    pub compression_ratio: f64,
    pub chunk_stats: ChunkStats,
}

impl FileInfo {
    pub fn print_summary(&self) {
        println!("HLC File Information:");
        println!("  Version: {}", self.version);
        println!("  Checksum: {:?}", self.checksum_type);
        println!("  Total chunks: {}", self.total_chunks);
        println!("  Original size: {} bytes ({:.2} MB)", 
                 self.original_size, 
                 self.original_size as f64 / (1024.0 * 1024.0));
        println!("  Compressed size: {} bytes ({:.2} MB)", 
                 self.compressed_size, 
                 self.compressed_size as f64 / (1024.0 * 1024.0));
        println!("  Compression ratio: {:.2}x", self.compression_ratio);
        println!("  Space saved: {:.1}%", self.chunk_stats.space_saved_percentage());
        
        println!("\nTransform Usage:");
        println!("  Stored (no compression): {}/{}", 
                 self.chunk_stats.stored_chunks, self.total_chunks);
        println!("  RLE encoding: {}/{}", 
                 self.chunk_stats.rle_chunks, self.total_chunks);
        println!("  Delta encoding: {}/{}", 
                 self.chunk_stats.delta_chunks, self.total_chunks);
        println!("  Dictionary compression: {}/{}", 
                 self.chunk_stats.dictionary_chunks, self.total_chunks);
        println!("  Entropy coding: {}/{}", 
                 self.chunk_stats.entropy_chunks, self.total_chunks);
    }
}

/// Estimate compression ratio without actually compressing
pub fn estimate_compression<R: Read>(
    reader: &mut R,
    config: &HlcConfig,
) -> Result<f32, HlcError> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    
    if buffer.is_empty() {
        return Ok(1.0);
    }

    let chunks = split_into_chunks(&buffer, config.chunk_size);
    let ratios: Vec<f32> = chunks
        .iter()
        .map(|chunk| crate::chunk::estimate_compression_ratio(chunk, config))
        .collect();

    // Weighted average based on chunk sizes
    let total_size: usize = chunks.iter().map(|c| c.size()).sum();
    let weighted_ratio: f32 = chunks
        .iter()
        .zip(ratios.iter())
        .map(|(chunk, &ratio)| ratio * (chunk.size() as f32 / total_size as f32))
        .sum();

    Ok(weighted_ratio)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_compress_decompress_roundtrip() {
        let original_data = b"Hello, world! This is a test of the HLC compression system.";
        let config = HlcConfig::default();

        // Compress
        let mut input = Cursor::new(original_data);
        let mut compressed = Vec::new();
        let stats = compress(&mut input, &mut compressed, &config).unwrap();

        assert!(stats.original_size > 0);
        assert!(stats.chunks_processed > 0);

        // Decompress
        let mut compressed_input = Cursor::new(compressed);
        let mut decompressed = Vec::new();
        decompress(&mut compressed_input, &mut decompressed, config.threads).unwrap();

        assert_eq!(original_data.to_vec(), decompressed);
    }

    #[test]
    fn test_empty_file_compression() {
        let original_data = b"";
        let config = HlcConfig::default();

        let mut input = Cursor::new(original_data);
        let mut compressed = Vec::new();
        let stats = compress(&mut input, &mut compressed, &config).unwrap();

        assert_eq!(stats.original_size, 0);
        assert_eq!(stats.chunks_processed, 0);

        let mut compressed_input = Cursor::new(compressed);
        let mut decompressed = Vec::new();
        decompress(&mut compressed_input, &mut decompressed, config.threads).unwrap();

        assert_eq!(original_data.to_vec(), decompressed);
    }

    #[test]
    fn test_file_info() {
        let original_data = vec![0u8; 1000]; // Highly compressible
        let config = HlcConfig::default();

        let mut input = Cursor::new(&original_data);
        let mut compressed = Vec::new();
        compress(&mut input, &mut compressed, &config).unwrap();

        let mut info_input = Cursor::new(&compressed);
        let info = info(&mut info_input).unwrap();

        assert_eq!(info.original_size, 1000);
        assert!(info.original_size > 0); // Just ensure it's valid
        assert!(info.compression_ratio > 0.0); // Just ensure it's valid
    }

    #[test]
    fn test_validation() {
        let original_data = b"Test data for validation";
        let config = HlcConfig::default();

        let mut input = Cursor::new(original_data);
        let mut compressed = Vec::new();
        compress(&mut input, &mut compressed, &config).unwrap();

        let mut validation_input = Cursor::new(compressed);
        let is_valid = validate(&mut validation_input).unwrap();
        assert!(is_valid);
    }
}