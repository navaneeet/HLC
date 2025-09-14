//! HLC (High-Level Compression) - A modern compression system with adaptive chunking
//! and neural metadata assist capabilities.

pub mod analyzer;
pub mod checksum;
pub mod chunker;
pub mod container;
pub mod entropy;
pub mod io;
pub mod transforms;
pub mod threadpool;

use anyhow::Result;
use std::path::Path;
use std::fs::File;
use std::io::{Read, Write};
use std::time::Instant;
use crate::chunker::AdaptiveChunker;
use crate::threadpool::RayonChunkProcessor;
use crate::container::GlobalHeader;

/// Compression modes for different speed/ratio tradeoffs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMode {
    Fast,
    Balanced,
    Max,
}

/// Configuration for HLC compression
#[derive(Debug, Clone)]
pub struct Config {
    pub mode: CompressionMode,
    pub threads: usize,
    pub enable_sha256: bool,
    pub enable_encryption: bool,
    pub chunk_size_min: usize,
    pub chunk_size_max: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: CompressionMode::Balanced,
            threads: num_cpus::get(),
            enable_sha256: false,
            enable_encryption: false,
            chunk_size_min: 1024,
            chunk_size_max: 64 * 1024,
        }
    }
}

/// Main HLC compressor
pub struct HLCCompressor {
    config: Config,
}

impl HLCCompressor {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Compress a file using HLC
    pub async fn compress_file<P: AsRef<Path>>(
        &self,
        input_path: P,
        output_path: P,
    ) -> Result<CompressionStats> {
        let start_time = Instant::now();
        
        // Read input file
        let mut input_file = File::open(&input_path)?;
        let mut input_data = Vec::new();
        input_file.read_to_end(&mut input_data)?;
        let original_size = input_data.len() as u64;
        
        // Create chunker
        let chunker = AdaptiveChunker::new(self.config.chunk_size_min, self.config.chunk_size_max);
        
        // Chunk the data
        let chunks = chunker.chunk_data(&input_data[..])?;
        let total_chunks = chunks.len() as u32;
        
        // Process chunks in parallel
        let processor = RayonChunkProcessor::new();
        let chunk_results = processor.process_chunks_parallel(chunks)?;
        
        // Calculate total compressed size
        let compressed_size: u64 = chunk_results.iter()
            .map(|result| result.compressed_data.len() as u64)
            .sum();
        
        // Write output file
        let mut output_file = File::create(&output_path)?;
        
        // Write global header
        let global_header = GlobalHeader::new(total_chunks, original_size, compressed_size);
        global_header.write(&mut output_file)?;
        
        // Write chunks in order
        for result in chunk_results {
            result.header.write(&mut output_file)?;
            output_file.write_all(&result.compressed_data)?;
        }
        
        let compression_time = start_time.elapsed().as_millis() as u64;
        
        Ok(CompressionStats::new(
            original_size,
            compressed_size,
            compression_time,
            total_chunks as usize,
        ))
    }

    /// Decompress a file using HLC
    pub async fn decompress_file<P: AsRef<Path>>(
        &self,
        _input_path: P,
        _output_path: P,
    ) -> Result<()> {
        // TODO: Implement decompression pipeline
        todo!("Decompression implementation")
    }
}

/// Statistics from compression operation
#[derive(Debug, Clone, serde::Serialize)]
pub struct CompressionStats {
    pub original_size: u64,
    pub compressed_size: u64,
    pub compression_ratio: f64,
    pub compression_time_ms: u64,
    pub chunks_processed: usize,
}

impl CompressionStats {
    pub fn new(original_size: u64, compressed_size: u64, compression_time_ms: u64, chunks_processed: usize) -> Self {
        Self {
            original_size,
            compressed_size,
            compression_ratio: compressed_size as f64 / original_size as f64,
            compression_time_ms,
            chunks_processed,
        }
    }
}

// Re-export num_cpus for convenience
pub use num_cpus;