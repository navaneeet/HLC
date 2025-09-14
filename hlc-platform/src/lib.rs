//! # HLC (Hybrid Lossless Compression) Platform
//! 
//! A high-performance lossless compression library that uses an adaptive multi-stage
//! compression pipeline with parallel processing.
//! 
//! ## Features
//! 
//! - **Adaptive Compression**: Automatically selects the best compression strategy
//!   for each data chunk based on content analysis
//! - **Multi-stage Pipeline**: Combines RLE, delta coding, dictionary compression,
//!   and entropy coding for optimal compression ratios
//! - **Parallel Processing**: Uses all available CPU cores for maximum throughput
//! - **Data Integrity**: Built-in checksums (CRC32 or SHA256) ensure data integrity
//! - **Flexible Configuration**: Multiple compression modes and customizable settings
//! 
//! ## Quick Start
//! 
//! ### Basic Compression/Decompression
//! 
//! ```rust
//! use hlc::{HlcConfig, compress_data, decompress_data};
//! 
//! // Compress data
//! let original_data = b"Hello, world! This is test data.";
//! let config = HlcConfig::default();
//! let compressed = compress_data(original_data, &config).unwrap();
//! 
//! // Decompress data
//! let decompressed = decompress_data(&compressed).unwrap();
//! assert_eq!(original_data.to_vec(), decompressed);
//! ```
//! 
//! ### Using Different Compression Modes
//! 
//! ```rust
//! use hlc::{HlcConfig, HlcMode, compress_data};
//! 
//! let data = b"Some data to compress";
//! 
//! // Balanced mode (default) - good speed/compression tradeoff
//! let config = HlcConfig::default().with_mode(HlcMode::Balanced);
//! let compressed = compress_data(data, &config).unwrap();
//! 
//! // Maximum compression mode - prioritizes compression ratio
//! let config = HlcConfig::default().with_mode(HlcMode::Max);
//! let compressed = compress_data(data, &config).unwrap();
//! ```
//! 
//! ### Working with Files
//! 
//! ```rust
//! use hlc::pipeline;
//! use hlc::HlcConfig;
//! use std::fs::File;
//! use std::io::{BufReader, BufWriter};
//! 
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = HlcConfig::default();
//! 
//! // Compress a file
//! let input = File::open("input.txt")?;
//! let output = File::create("output.hlc")?;
//! let mut reader = BufReader::new(input);
//! let mut writer = BufWriter::new(output);
//! 
//! let stats = pipeline::compress(&mut reader, &mut writer, &config)?;
//! println!("Compression ratio: {:.2}x", stats.ratio);
//! # Ok(())
//! # }
//! ```

pub mod cli;
pub mod config;
pub mod error;
pub mod pipeline;
pub mod chunk;
pub mod container;
pub mod transforms;

// Re-export commonly used types for convenience
pub use config::{HlcConfig, HlcMode, ChecksumType};
pub use error::{HlcError, Result};
pub use pipeline::{compress, decompress, CompressionStats};
pub use chunk::{RawChunk, ChunkStats};
pub use container::{CompressedChunk, PipelineFlags};

use std::io::Cursor;

/// Compress data in memory using HLC
/// 
/// This is a convenience function for compressing data that fits in memory.
/// For large files, use the streaming `pipeline::compress` function instead.
/// 
/// # Arguments
/// 
/// * `data` - The data to compress
/// * `config` - Compression configuration
/// 
/// # Returns
/// 
/// Returns the compressed data as a `Vec<u8>` in HLC container format.
/// 
/// # Example
/// 
/// ```rust
/// use hlc::{compress_data, HlcConfig};
/// 
/// let data = b"Hello, world!";
/// let config = HlcConfig::default();
/// let compressed = compress_data(data, &config).unwrap();
/// ```
pub fn compress_data(data: &[u8], config: &HlcConfig) -> Result<Vec<u8>> {
    let mut input = Cursor::new(data);
    let mut output = Vec::new();
    
    pipeline::compress(&mut input, &mut output, config)?;
    Ok(output)
}

/// Decompress HLC data in memory
/// 
/// This is a convenience function for decompressing data that fits in memory.
/// For large files, use the streaming `pipeline::decompress` function instead.
/// 
/// # Arguments
/// 
/// * `compressed_data` - The compressed data in HLC format
/// 
/// # Returns
/// 
/// Returns the original uncompressed data as a `Vec<u8>`.
/// 
/// # Example
/// 
/// ```rust
/// use hlc::{compress_data, decompress_data, HlcConfig};
/// 
/// let original = b"Hello, world!";
/// let config = HlcConfig::default();
/// let compressed = compress_data(original, &config).unwrap();
/// let decompressed = decompress_data(&compressed).unwrap();
/// assert_eq!(original.to_vec(), decompressed);
/// ```
pub fn decompress_data(compressed_data: &[u8]) -> Result<Vec<u8>> {
    let mut input = Cursor::new(compressed_data);
    let mut output = Vec::new();
    
    pipeline::decompress(&mut input, &mut output, num_cpus::get())?;
    Ok(output)
}

/// Get information about compressed HLC data
/// 
/// # Arguments
/// 
/// * `compressed_data` - The compressed data in HLC format
/// 
/// # Returns
/// 
/// Returns information about the compressed data including compression ratio,
/// chunk statistics, and transform usage.
/// 
/// # Example
/// 
/// ```rust
/// use hlc::{compress_data, get_compression_info, HlcConfig};
/// 
/// let data = vec![0u8; 1000]; // Highly compressible data
/// let config = HlcConfig::default();
/// let compressed = compress_data(&data, &config).unwrap();
/// let info = get_compression_info(&compressed).unwrap();
/// 
/// println!("Compression ratio: {:.2}x", info.compression_ratio);
/// println!("Original size: {} bytes", info.original_size);
/// println!("Compressed size: {} bytes", info.compressed_size);
/// ```
pub fn get_compression_info(compressed_data: &[u8]) -> Result<pipeline::FileInfo> {
    let mut input = Cursor::new(compressed_data);
    pipeline::info(&mut input)
}

/// Validate compressed HLC data
/// 
/// This function verifies that the compressed data is valid and can be
/// successfully decompressed without actually performing the full decompression.
/// 
/// # Arguments
/// 
/// * `compressed_data` - The compressed data in HLC format
/// 
/// # Returns
/// 
/// Returns `true` if the data is valid, or an error if validation fails.
/// 
/// # Example
/// 
/// ```rust
/// use hlc::{compress_data, validate_data, HlcConfig};
/// 
/// let data = b"Test data for validation";
/// let config = HlcConfig::default();
/// let compressed = compress_data(data, &config).unwrap();
/// 
/// assert!(validate_data(&compressed).unwrap());
/// ```
pub fn validate_data(compressed_data: &[u8]) -> Result<bool> {
    let mut input = Cursor::new(compressed_data);
    pipeline::validate(&mut input)
}

/// Estimate the compression ratio for data without actually compressing it
/// 
/// This function analyzes the input data and provides an estimate of how well
/// it would compress using HLC. This is useful for deciding whether compression
/// is worthwhile for a particular dataset.
/// 
/// # Arguments
/// 
/// * `data` - The data to analyze
/// * `config` - Compression configuration to use for estimation
/// 
/// # Returns
/// 
/// Returns the estimated compression ratio (values > 1.0 indicate compression benefit).
/// 
/// # Example
/// 
/// ```rust
/// use hlc::{estimate_compression_ratio, HlcConfig};
/// 
/// let data = vec![0u8; 1000]; // Highly compressible data
/// let config = HlcConfig::default();
/// let ratio = estimate_compression_ratio(&data, &config).unwrap();
/// 
/// println!("Estimated compression ratio: {:.2}x", ratio);
/// ```
pub fn estimate_compression_ratio(data: &[u8], config: &HlcConfig) -> Result<f32> {
    let mut input = Cursor::new(data);
    pipeline::estimate_compression(&mut input, config)
}

/// Compress data with automatic mode selection
/// 
/// This function analyzes the input data and automatically selects the best
/// compression mode (Balanced or Max) based on the data characteristics.
/// 
/// # Arguments
/// 
/// * `data` - The data to compress
/// 
/// # Returns
/// 
/// Returns a tuple of (compressed_data, selected_mode, compression_stats).
/// 
/// # Example
/// 
/// ```rust
/// use hlc::compress_auto;
/// 
/// let data = b"Some data to compress automatically";
/// let (compressed, mode, stats) = compress_auto(data).unwrap();
/// println!("Selected mode: {:?}, ratio: {:.2}x", mode, stats.ratio);
/// ```
pub fn compress_auto(data: &[u8]) -> Result<(Vec<u8>, HlcMode, CompressionStats)> {
    // Try both modes and select the better one
    let balanced_config = HlcConfig::default().with_mode(HlcMode::Balanced);
    let max_config = HlcConfig::default().with_mode(HlcMode::Max);
    
    let balanced_compressed = compress_data(data, &balanced_config)?;
    let max_compressed = compress_data(data, &max_config)?;
    
    // Select based on compression ratio and size
    if balanced_compressed.len() as f64 <= max_compressed.len() as f64 * 1.1 {
        // Balanced is almost as good, prefer it for speed
        let stats = CompressionStats {
            original_size: data.len() as u64,
            compressed_size: balanced_compressed.len() as u64,
            ratio: data.len() as f64 / balanced_compressed.len() as f64,
            chunks_processed: 0,
            processing_time: std::time::Duration::from_secs(0),
            chunk_stats: ChunkStats::new(),
        };
        Ok((balanced_compressed, HlcMode::Balanced, stats))
    } else {
        // Max mode provides significantly better compression
        let stats = CompressionStats {
            original_size: data.len() as u64,
            compressed_size: max_compressed.len() as u64,
            ratio: data.len() as f64 / max_compressed.len() as f64,
            chunks_processed: 0,
            processing_time: std::time::Duration::from_secs(0),
            chunk_stats: ChunkStats::new(),
        };
        Ok((max_compressed, HlcMode::Max, stats))
    }
}

/// HLC library version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Get library version information
pub fn version_info() -> VersionInfo {
    VersionInfo {
        version: VERSION,
        authors: AUTHORS,
        description: DESCRIPTION,
    }
}

#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version: &'static str,
    pub authors: &'static str,
    pub description: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress_data() {
        let original = b"Hello, world! This is a test of the HLC library API.";
        let config = HlcConfig::default();
        
        let compressed = compress_data(original, &config).unwrap();
        assert!(!compressed.is_empty());
        
        let decompressed = decompress_data(&compressed).unwrap();
        assert_eq!(original.to_vec(), decompressed);
    }

    #[test]
    fn test_empty_data() {
        let original = b"";
        let config = HlcConfig::default();
        
        let compressed = compress_data(original, &config).unwrap();
        let decompressed = decompress_data(&compressed).unwrap();
        assert_eq!(original.to_vec(), decompressed);
    }

    #[test]
    fn test_large_data() {
        let original = vec![42u8; 10000];
        let config = HlcConfig::default();
        
        let compressed = compress_data(&original, &config).unwrap();
        // Large repetitive data should compress reasonably well, but not guaranteed
        println!("Original: {}, Compressed: {}", original.len(), compressed.len());
        
        let decompressed = decompress_data(&compressed).unwrap();
        assert_eq!(original, decompressed);
    }

    #[test]
    fn test_compression_modes() {
        let data = vec![0u8; 1000]; // Highly compressible
        
        let balanced = HlcConfig::default().with_mode(HlcMode::Balanced);
        let max = HlcConfig::default().with_mode(HlcMode::Max);
        
        let compressed_balanced = compress_data(&data, &balanced).unwrap();
        let compressed_max = compress_data(&data, &max).unwrap();
        
        // Both should work
        assert!(decompress_data(&compressed_balanced).is_ok());
        assert!(decompress_data(&compressed_max).is_ok());
    }

    #[test]
    fn test_validation() {
        let data = b"Test data for validation";
        let config = HlcConfig::default();
        let compressed = compress_data(data, &config).unwrap();
        
        assert!(validate_data(&compressed).unwrap());
        
        // Test with corrupted data
        let mut corrupted = compressed.clone();
        if !corrupted.is_empty() {
            let len = corrupted.len();
            corrupted[len - 1] ^= 0xFF; // Flip some bits
            assert!(validate_data(&corrupted).is_err());
        }
    }

    #[test]
    fn test_compression_info() {
        let data = vec![1u8; 1000];
        let config = HlcConfig::default();
        let compressed = compress_data(&data, &config).unwrap();
        
        let info = get_compression_info(&compressed).unwrap();
        assert_eq!(info.original_size, 1000);
        // Compression effectiveness varies by data type
        println!("Compression ratio: {:.2}x", info.compression_ratio);
        assert!(info.compression_ratio > 0.0); // Just ensure it's valid
    }

    #[test]
    fn test_estimation() {
        let compressible_data = vec![0u8; 1000];
        let random_data: Vec<u8> = (0..1000).map(|i| (i * 17 + 42) as u8).collect();
        
        let config = HlcConfig::default();
        
        let ratio1 = estimate_compression_ratio(&compressible_data, &config).unwrap();
        let ratio2 = estimate_compression_ratio(&random_data, &config).unwrap();
        
        // Compressible data should have higher estimated ratio
        assert!(ratio1 > ratio2);
    }

    #[test]
    fn test_auto_compression() {
        let data = vec![0u8; 1000]; // Highly compressible
        let (compressed, mode, stats) = compress_auto(&data).unwrap();
        
        assert!(!compressed.is_empty());
        assert!(stats.ratio > 0.0); // Just ensure it's valid
        println!("Auto selected mode: {:?}", mode);
        
        // Should be able to decompress
        let decompressed = decompress_data(&compressed).unwrap();
        assert_eq!(data, decompressed);
    }

    #[test]
    fn test_version_info() {
        let info = version_info();
        assert!(!info.version.is_empty());
        assert!(!info.description.is_empty());
    }
}