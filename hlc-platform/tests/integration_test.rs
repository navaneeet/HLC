//! Integration tests for the HLC platform
//! 
//! These tests verify the complete end-to-end functionality of the compression system.

use hlc::{HlcConfig, HlcMode, ChecksumType};
use hlc::{compress_data, decompress_data, validate_data, get_compression_info, estimate_compression_ratio};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use tempfile::{NamedTempFile, TempDir};

#[test]
fn test_basic_compression_roundtrip() {
    let original_data = b"Hello, world! This is a basic compression test.";
    let config = HlcConfig::default();
    
    let compressed = compress_data(original_data, &config).unwrap();
    let decompressed = decompress_data(&compressed).unwrap();
    
    assert_eq!(original_data.to_vec(), decompressed);
}

#[test]
fn test_empty_data_handling() {
    let empty_data = b"";
    let config = HlcConfig::default();
    
    let compressed = compress_data(empty_data, &config).unwrap();
    let decompressed = decompress_data(&compressed).unwrap();
    
    assert_eq!(empty_data.to_vec(), decompressed);
    assert!(validate_data(&compressed).unwrap());
}

#[test]
fn test_large_data_compression() {
    // Create a large dataset with patterns that should compress well
    let mut large_data = Vec::with_capacity(1_000_000);
    
    // Add some structured data
    for i in 0..10000 {
        large_data.extend_from_slice(&(i as u32).to_le_bytes());
        large_data.extend_from_slice(b"PADDING");
        large_data.extend_from_slice(&[0u8; 90]); // Sparse data
    }
    
    let config = HlcConfig::default();
    let compressed = compress_data(&large_data, &config).unwrap();
    let decompressed = decompress_data(&compressed).unwrap();
    
    assert_eq!(large_data, decompressed);
    assert!(compressed.len() < large_data.len());
    
    let info = get_compression_info(&compressed).unwrap();
    assert!(info.compression_ratio > 1.0);
    println!("Large data compression ratio: {:.2}x", info.compression_ratio);
}

#[test]
fn test_different_compression_modes() {
    let test_data = create_test_data(10000);
    
    let balanced_config = HlcConfig::default().with_mode(HlcMode::Balanced);
    let max_config = HlcConfig::default().with_mode(HlcMode::Max);
    
    let compressed_balanced = compress_data(&test_data, &balanced_config).unwrap();
    let compressed_max = compress_data(&test_data, &max_config).unwrap();
    
    // Both should decompress correctly
    let decompressed_balanced = decompress_data(&compressed_balanced).unwrap();
    let decompressed_max = decompress_data(&compressed_max).unwrap();
    
    assert_eq!(test_data, decompressed_balanced);
    assert_eq!(test_data, decompressed_max);
    
    // Get compression stats
    let info_balanced = get_compression_info(&compressed_balanced).unwrap();
    let info_max = get_compression_info(&compressed_max).unwrap();
    
    println!("Balanced mode: {:.2}x ratio", info_balanced.compression_ratio);
    println!("Max mode: {:.2}x ratio", info_max.compression_ratio);
    
    // Max mode should generally achieve better compression (though not guaranteed)
    assert!(info_max.compression_ratio >= info_balanced.compression_ratio * 0.9);
}

#[test]
fn test_different_checksum_types() {
    let test_data = b"Test data for checksum validation";
    
    let crc32_config = HlcConfig::default().with_checksum(ChecksumType::CRC32);
    let sha256_config = HlcConfig::default().with_checksum(ChecksumType::SHA256);
    
    let compressed_crc32 = compress_data(test_data, &crc32_config).unwrap();
    let compressed_sha256 = compress_data(test_data, &sha256_config).unwrap();
    
    // Both should work correctly
    let decompressed_crc32 = decompress_data(&compressed_crc32).unwrap();
    let decompressed_sha256 = decompress_data(&compressed_sha256).unwrap();
    
    assert_eq!(test_data.to_vec(), decompressed_crc32);
    assert_eq!(test_data.to_vec(), decompressed_sha256);
    
    // Both should validate
    assert!(validate_data(&compressed_crc32).unwrap());
    assert!(validate_data(&compressed_sha256).unwrap());
}

#[test]
fn test_file_compression_decompression() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.txt");
    let compressed_path = temp_dir.path().join("compressed.hlc");
    let output_path = temp_dir.path().join("output.txt");
    
    // Create test input file
    let test_data = create_test_data(50000);
    std::fs::write(&input_path, &test_data).unwrap();
    
    let config = HlcConfig::default();
    
    // Compress file
    {
        let input_file = File::open(&input_path).unwrap();
        let output_file = File::create(&compressed_path).unwrap();
        let mut reader = BufReader::new(input_file);
        let mut writer = BufWriter::new(output_file);
        
        let stats = hlc::pipeline::compress(&mut reader, &mut writer, &config).unwrap();
        assert!(stats.ratio > 1.0);
        println!("File compression ratio: {:.2}x", stats.ratio);
    }
    
    // Decompress file
    {
        let input_file = File::open(&compressed_path).unwrap();
        let output_file = File::create(&output_path).unwrap();
        let mut reader = BufReader::new(input_file);
        let mut writer = BufWriter::new(output_file);
        
        hlc::pipeline::decompress(&mut reader, &mut writer, 4).unwrap();
    }
    
    // Verify the result
    let decompressed_data = std::fs::read(&output_path).unwrap();
    assert_eq!(test_data, decompressed_data);
}

#[test]
fn test_multithreaded_compression() {
    let test_data = create_test_data(100000);
    
    // Test with different thread counts
    for threads in [1, 2, 4, 8] {
        let config = HlcConfig::default().with_threads(threads);
        let compressed = compress_data(&test_data, &config).unwrap();
        let decompressed = decompress_data(&compressed).unwrap();
        
        assert_eq!(test_data, decompressed);
        println!("Threads: {}, compressed size: {}", threads, compressed.len());
    }
}

#[test]
fn test_chunk_size_variations() {
    let test_data = create_test_data(100000);
    
    // Test with different chunk sizes
    for chunk_size in [1024, 4096, 16384, 65536, 262144] {
        let config = HlcConfig::default().with_chunk_size(chunk_size);
        let compressed = compress_data(&test_data, &config).unwrap();
        let decompressed = decompress_data(&compressed).unwrap();
        
        assert_eq!(test_data, decompressed);
        
        let info = get_compression_info(&compressed).unwrap();
        println!("Chunk size: {}, ratio: {:.2}x, chunks: {}", 
                 chunk_size, info.compression_ratio, info.total_chunks);
    }
}

#[test]
fn test_data_corruption_detection() {
    let test_data = b"Test data for corruption detection";
    let config = HlcConfig::default();
    
    let mut compressed = compress_data(test_data, &config).unwrap();
    
    // Corrupt the data by flipping some bits
    if compressed.len() > 10 {
        compressed[compressed.len() - 5] ^= 0xFF;
        
        // Should detect corruption during decompression
        assert!(decompress_data(&compressed).is_err());
        assert!(validate_data(&compressed).is_err());
    }
}

#[test]
fn test_compression_estimation() {
    let highly_compressible = vec![0u8; 10000];
    let less_compressible: Vec<u8> = (0..10000).map(|i| (i * 17 + 42) as u8).collect();
    
    let config = HlcConfig::default();
    
    let ratio1 = estimate_compression_ratio(&highly_compressible, &config).unwrap();
    let ratio2 = estimate_compression_ratio(&less_compressible, &config).unwrap();
    
    println!("Estimated ratios: {:.2}x vs {:.2}x", ratio1, ratio2);
    
    // Highly compressible data should have a higher estimated ratio
    assert!(ratio1 > ratio2);
    
    // Verify estimates are reasonable by actual compression
    let actual1 = compress_data(&highly_compressible, &config).unwrap();
    let actual_ratio1 = highly_compressible.len() as f32 / actual1.len() as f32;
    
    // Estimate should be in the right ballpark (within 2x)
    assert!(ratio1 > actual_ratio1 * 0.5);
    assert!(ratio1 < actual_ratio1 * 2.0);
}

#[test]
fn test_various_data_types() {
    let test_cases = vec![
        ("Text data", b"The quick brown fox jumps over the lazy dog. ".repeat(100).into_bytes()),
        ("Binary data", (0..1000u16).flat_map(|i| i.to_le_bytes()).collect()),
        ("Sparse data", {
            let mut data = vec![0u8; 1000];
            for i in (0..1000).step_by(100) {
                data[i] = 255;
            }
            data
        }),
        ("Random data", (0..1000).map(|i| (i * 17 + 42) as u8).collect()),
        ("Repeated patterns", b"ABCDEFGH".repeat(125)),
    ];
    
    for (name, data) in test_cases {
        let config = HlcConfig::default();
        let compressed = compress_data(&data, &config).unwrap();
        let decompressed = decompress_data(&compressed).unwrap();
        
        assert_eq!(data, decompressed);
        
        let ratio = data.len() as f64 / compressed.len() as f64;
        println!("{}: {:.2}x compression ratio", name, ratio);
        
        assert!(validate_data(&compressed).unwrap());
    }
}

#[test]
fn test_pipeline_info_and_stats() {
    let test_data = create_test_data(50000);
    let config = HlcConfig::default();
    
    let compressed = compress_data(&test_data, &config).unwrap();
    let info = get_compression_info(&compressed).unwrap();
    
    // Verify info structure
    assert_eq!(info.original_size, test_data.len() as u64);
    assert!(info.compressed_size > 0);
    assert!(info.compression_ratio > 0.0);
    assert!(info.total_chunks > 0);
    
    // Print detailed information
    info.print_summary();
    
    // Check chunk statistics
    let total_chunks = info.chunk_stats.total_chunks;
    assert_eq!(total_chunks, info.total_chunks);
    
    let transform_usage = info.chunk_stats.stored_chunks + 
                         info.chunk_stats.rle_chunks + 
                         info.chunk_stats.delta_chunks + 
                         info.chunk_stats.dictionary_chunks + 
                         info.chunk_stats.entropy_chunks;
    
    // Note: chunks can have multiple transforms, so this isn't a simple equality
    assert!(transform_usage >= total_chunks);
}

/// Helper function to create test data with various patterns
fn create_test_data(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    
    // Add different types of patterns
    let pattern_size = size / 4;
    
    // Sequential data (good for delta compression)
    for i in 0..pattern_size {
        data.push((i % 256) as u8);
    }
    
    // Sparse data (good for RLE)
    for i in 0..pattern_size {
        if i % 10 == 0 {
            data.push(255);
        } else {
            data.push(0);
        }
    }
    
    // Repeated patterns (good for dictionary compression)
    let pattern = b"PATTERN";
    for _ in 0..(pattern_size / pattern.len()) {
        data.extend_from_slice(pattern);
    }
    
    // Fill remaining space with mixed data
    while data.len() < size {
        data.push(((data.len() * 17 + 42) % 256) as u8);
    }
    
    data.truncate(size);
    data
}

#[test]
fn test_error_conditions() {
    // Test with invalid HLC data
    let invalid_data = b"This is not HLC data";
    assert!(decompress_data(invalid_data).is_err());
    assert!(validate_data(invalid_data).is_err());
    assert!(get_compression_info(invalid_data).is_err());
    
    // Test with truncated HLC data
    let valid_data = b"Test data";
    let config = HlcConfig::default();
    let mut compressed = compress_data(valid_data, &config).unwrap();
    
    if compressed.len() > 5 {
        compressed.truncate(compressed.len() / 2); // Truncate
        assert!(decompress_data(&compressed).is_err());
        assert!(validate_data(&compressed).is_err());
    }
}

#[test]
fn test_auto_compression() {
    let test_data = create_test_data(10000);
    let (compressed, selected_mode, stats) = hlc::compress_auto(&test_data).unwrap();
    
    println!("Auto-selected mode: {:?}", selected_mode);
    println!("Auto compression ratio: {:.2}x", stats.ratio);
    
    let decompressed = decompress_data(&compressed).unwrap();
    assert_eq!(test_data, decompressed);
    
    assert!(stats.ratio > 0.0);
    assert!(matches!(selected_mode, HlcMode::Balanced | HlcMode::Max));
}

#[test]
fn test_version_info() {
    let version_info = hlc::version_info();
    assert!(!version_info.version.is_empty());
    assert!(!version_info.description.is_empty());
    
    println!("HLC Version: {}", version_info.version);
    println!("Description: {}", version_info.description);
}