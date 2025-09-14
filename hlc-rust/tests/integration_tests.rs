//! Integration tests for HLC compression system

use hlc_rust::{HLCCompressor, Config, CompressionMode};
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

#[tokio::test]
async fn test_compress_small_text_file() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.txt");
    let output_path = temp_dir.path().join("output.hlc");
    
    // Create test file
    let mut input_file = File::create(&input_path).unwrap();
    input_file.write_all(b"Hello, world! This is a test of HLC compression.").unwrap();
    
    // Compress
    let config = Config::default();
    let compressor = HLCCompressor::new(config);
    let stats = compressor.compress_file(&input_path, &output_path).await.unwrap();
    
    // Verify compression
    assert!(stats.compressed_size > 0);
    assert!(stats.compression_ratio < 1.0);
    assert!(stats.chunks_processed > 0);
    
    // Verify output file exists
    assert!(output_path.exists());
}

#[tokio::test]
async fn test_compress_json_file() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.json");
    let output_path = temp_dir.path().join("output.hlc");
    
    // Create JSON test file
    let json_data = r#"{
        "name": "test",
        "value": 42,
        "items": [1, 2, 3, 4, 5],
        "nested": {
            "key": "value",
            "number": 3.14
        }
    }"#;
    
    let mut input_file = File::create(&input_path).unwrap();
    input_file.write_all(json_data.as_bytes()).unwrap();
    
    // Compress
    let config = Config::default();
    let compressor = HLCCompressor::new(config);
    let stats = compressor.compress_file(&input_path, &output_path).await.unwrap();
    
    // Verify compression
    assert!(stats.compressed_size > 0);
    assert!(stats.compression_ratio < 1.0);
    assert!(stats.chunks_processed > 0);
}

#[tokio::test]
async fn test_compress_repeated_data() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.txt");
    let output_path = temp_dir.path().join("output.hlc");
    
    // Create file with repeated data (should compress well)
    let repeated_data = "Hello, world! ".repeat(1000);
    let mut input_file = File::create(&input_path).unwrap();
    input_file.write_all(repeated_data.as_bytes()).unwrap();
    
    // Compress
    let config = Config::default();
    let compressor = HLCCompressor::new(config);
    let stats = compressor.compress_file(&input_path, &output_path).await.unwrap();
    
    // Should achieve good compression ratio
    assert!(stats.compression_ratio < 0.5);
    assert!(stats.compressed_size < stats.original_size);
}

#[tokio::test]
async fn test_compress_different_modes() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.txt");
    
    // Create test file
    let test_data = "This is a test file for compression mode testing. ".repeat(100);
    let mut input_file = File::create(&input_path).unwrap();
    input_file.write_all(test_data.as_bytes()).unwrap();
    
    let modes = [CompressionMode::Fast, CompressionMode::Balanced, CompressionMode::Max];
    let mut results = Vec::new();
    
    for mode in modes {
        let output_path = temp_dir.path().join(format!("output_{:?}.hlc", mode));
        let config = Config {
            mode,
            ..Default::default()
        };
        let compressor = HLCCompressor::new(config);
        let stats = compressor.compress_file(&input_path, &output_path).await.unwrap();
        results.push((mode, stats));
    }
    
    // All modes should produce valid compressed files
    for (mode, stats) in &results {
        assert!(stats.compressed_size > 0);
        assert!(stats.compression_ratio < 1.0);
        println!("Mode {:?}: ratio = {:.2}%", mode, stats.compression_ratio * 100.0);
    }
}

#[tokio::test]
async fn test_compress_large_file() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.txt");
    let output_path = temp_dir.path().join("output.hlc");
    
    // Create a larger test file (1MB)
    let mut input_file = File::create(&input_path).unwrap();
    let _chunk_size = 1024;
    let num_chunks = 1024; // 1MB total
    
    for i in 0..num_chunks {
        let chunk_data = format!("Chunk {}: This is test data for compression. ", i).repeat(10);
        input_file.write_all(chunk_data.as_bytes()).unwrap();
    }
    
    // Compress
    let config = Config::default();
    let compressor = HLCCompressor::new(config);
    let stats = compressor.compress_file(&input_path, &output_path).await.unwrap();
    
    // Verify compression
    assert!(stats.original_size > 1_000_000); // At least 1MB
    assert!(stats.compressed_size > 0);
    assert!(stats.compression_ratio < 1.0);
    assert!(stats.chunks_processed > 0);
}

#[tokio::test]
async fn test_compress_binary_data() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.bin");
    let output_path = temp_dir.path().join("output.hlc");
    
    // Create binary test file
    let mut input_file = File::create(&input_path).unwrap();
    let binary_data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
    input_file.write_all(&binary_data).unwrap();
    
    // Compress
    let config = Config::default();
    let compressor = HLCCompressor::new(config);
    let stats = compressor.compress_file(&input_path, &output_path).await.unwrap();
    
    // Verify compression
    assert!(stats.compressed_size > 0);
    assert!(stats.compression_ratio < 1.0);
    assert!(stats.chunks_processed > 0);
}

#[tokio::test]
async fn test_compress_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.txt");
    let output_path = temp_dir.path().join("output.hlc");
    
    // Create empty file
    File::create(&input_path).unwrap();
    
    // Compress
    let config = Config::default();
    let compressor = HLCCompressor::new(config);
    let stats = compressor.compress_file(&input_path, &output_path).await.unwrap();
    
    // Verify compression
    assert_eq!(stats.original_size, 0);
    assert_eq!(stats.compressed_size, 0);
    assert_eq!(stats.chunks_processed, 0);
}

#[tokio::test]
async fn test_compress_single_byte() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.txt");
    let output_path = temp_dir.path().join("output.hlc");
    
    // Create single byte file
    let mut input_file = File::create(&input_path).unwrap();
    input_file.write_all(b"A").unwrap();
    
    // Compress
    let config = Config::default();
    let compressor = HLCCompressor::new(config);
    let stats = compressor.compress_file(&input_path, &output_path).await.unwrap();
    
    // Verify compression
    assert_eq!(stats.original_size, 1);
    assert!(stats.compressed_size > 0);
    assert!(stats.chunks_processed > 0);
}