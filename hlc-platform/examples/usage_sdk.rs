//! Example demonstrating how to use HLC as a library/SDK
//! 
//! This example shows various ways to use the HLC compression library
//! in your own Rust applications.

use hlc::{
    HlcConfig, HlcMode, ChecksumType,
    compress_data, decompress_data, validate_data, get_compression_info,
    estimate_compression_ratio, compress_auto, pipeline
};
use std::fs::File;
use std::io::{BufReader, BufWriter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("HLC SDK Usage Examples");
    println!("=====================");
    
    // Example 1: Basic in-memory compression
    basic_compression_example()?;
    
    // Example 2: Different compression modes
    compression_modes_example()?;
    
    // Example 3: Configuration options
    configuration_example()?;
    
    // Example 4: File compression with streaming
    file_compression_example()?;
    
    // Example 5: Data analysis and estimation
    analysis_example()?;
    
    // Example 6: Error handling and validation
    validation_example()?;
    
    // Example 7: Advanced features
    advanced_features_example()?;
    
    Ok(())
}

fn basic_compression_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n1. Basic Compression Example");
    println!("----------------------------");
    
    let original_data = b"Hello, world! This is a simple compression example using HLC.";
    println!("Original data: {:?}", std::str::from_utf8(original_data)?);
    println!("Original size: {} bytes", original_data.len());
    
    // Compress using default configuration
    let config = HlcConfig::default();
    let compressed = compress_data(original_data, &config)?;
    println!("Compressed size: {} bytes", compressed.len());
    
    // Decompress
    let decompressed = decompress_data(&compressed)?;
    println!("Decompressed data: {:?}", std::str::from_utf8(&decompressed)?);
    
    // Verify integrity
    assert_eq!(original_data.to_vec(), decompressed);
    println!("✓ Data integrity verified");
    
    let ratio = original_data.len() as f64 / compressed.len() as f64;
    println!("Compression ratio: {:.2}x", ratio);
    
    Ok(())
}

fn compression_modes_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n2. Compression Modes Example");
    println!("----------------------------");
    
    // Create some test data that should compress well
    let mut test_data = Vec::new();
    test_data.extend(b"REPEATED_PATTERN".repeat(100));
    test_data.extend(vec![0u8; 500]); // Sparse data
    test_data.extend((0u8..255).cycle().take(1000).collect::<Vec<u8>>()); // Sequential data
    
    println!("Test data size: {} bytes", test_data.len());
    
    // Try different compression modes
    let modes = vec![
        ("Balanced", HlcMode::Balanced),
        ("Maximum", HlcMode::Max),
    ];
    
    for (name, mode) in modes {
        let config = HlcConfig::default().with_mode(mode);
        let compressed = compress_data(&test_data, &config)?;
        let ratio = test_data.len() as f64 / compressed.len() as f64;
        
        println!("{} mode: {} bytes compressed, {:.2}x ratio", name, compressed.len(), ratio);
        
        // Verify decompression works
        let decompressed = decompress_data(&compressed)?;
        assert_eq!(test_data, decompressed);
    }
    
    // Try automatic mode selection
    let (compressed, selected_mode, stats) = compress_auto(&test_data)?;
    println!("Auto-selected mode: {:?}", selected_mode);
    println!("Auto compression: {} bytes, {:.2}x ratio", compressed.len(), stats.ratio);
    
    Ok(())
}

fn configuration_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n3. Configuration Example");
    println!("------------------------");
    
    let test_data = b"Configuration test data with various settings".repeat(50);
    
    // Create different configurations
    let configs = vec![
        ("Default", HlcConfig::default()),
        ("Fast with CRC32", HlcConfig::default()
            .with_mode(HlcMode::Balanced)
            .with_checksum(ChecksumType::CRC32)
            .with_threads(1)),
        ("Max compression with SHA256", HlcConfig::default()
            .with_mode(HlcMode::Max)
            .with_checksum(ChecksumType::SHA256)
            .with_threads(4)),
        ("Small chunks", HlcConfig::default()
            .with_chunk_size(1024)),
        ("Large chunks", HlcConfig::default()
            .with_chunk_size(64 * 1024)),
    ];
    
    for (name, config) in configs {
        let compressed = compress_data(&test_data, &config)?;
        let info = get_compression_info(&compressed)?;
        
        println!("{}: {} bytes, {:.2}x ratio, {} chunks", 
                 name, compressed.len(), info.compression_ratio, info.total_chunks);
        
        // Verify decompression
        let decompressed = decompress_data(&compressed)?;
        assert_eq!(test_data.to_vec(), decompressed);
    }
    
    Ok(())
}

fn file_compression_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n4. File Compression Example");
    println!("---------------------------");
    
    // Create a temporary file with test data
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    let mut input_file = NamedTempFile::new()?;
    let test_content = "Line of text data\n".repeat(1000);
    input_file.write_all(test_content.as_bytes())?;
    input_file.flush()?;
    
    let compressed_file = NamedTempFile::new()?;
    let output_file = NamedTempFile::new()?;
    
    let config = HlcConfig::default().with_mode(HlcMode::Balanced);
    
    // Compress file using streaming API
    {
        let input = File::open(input_file.path())?;
        let output = File::create(compressed_file.path())?;
        let mut reader = BufReader::new(input);
        let mut writer = BufWriter::new(output);
        
        let stats = pipeline::compress(&mut reader, &mut writer, &config)?;
        println!("File compressed: {:.2}x ratio, {} chunks processed", 
                 stats.ratio, stats.chunks_processed);
        println!("Processing time: {:?}", stats.processing_time);
        println!("Throughput: {:.2} MB/s", stats.throughput_mbps());
    }
    
    // Decompress file
    {
        let input = File::open(compressed_file.path())?;
        let output = File::create(output_file.path())?;
        let mut reader = BufReader::new(input);
        let mut writer = BufWriter::new(output);
        
        pipeline::decompress(&mut reader, &mut writer, config.threads)?;
        println!("File decompressed successfully");
    }
    
    // Verify the result
    let original_content = std::fs::read_to_string(input_file.path())?;
    let decompressed_content = std::fs::read_to_string(output_file.path())?;
    assert_eq!(original_content, decompressed_content);
    println!("✓ File integrity verified");
    
    Ok(())
}

fn analysis_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n5. Data Analysis Example");
    println!("------------------------");
    
    let data_types = vec![
        ("Text data", "The quick brown fox jumps over the lazy dog. ".repeat(100).into_bytes()),
        ("Binary data", (0..1000u32).flat_map(|i| i.to_le_bytes()).collect()),
        ("Sparse data", {
            let mut data = vec![0u8; 1000];
            for i in (0..1000).step_by(50) {
                data[i] = 255;
            }
            data
        }),
        ("Random data", (0..1000).map(|i| ((i * 17 + 42) % 256) as u8).collect()),
    ];
    
    let config = HlcConfig::default();
    
    for (name, data) in data_types {
        // Estimate compression ratio
        let estimated_ratio = estimate_compression_ratio(&data, &config)?;
        
        // Actual compression
        let compressed = compress_data(&data, &config)?;
        let actual_ratio = data.len() as f32 / compressed.len() as f32;
        
        // Get detailed info
        let info = get_compression_info(&compressed)?;
        
        println!("{}: estimated {:.2}x, actual {:.2}x", name, estimated_ratio, actual_ratio);
        println!("  Transforms used: RLE:{}, Delta:{}, Dict:{}, Entropy:{}", 
                 info.chunk_stats.rle_chunks,
                 info.chunk_stats.delta_chunks,
                 info.chunk_stats.dictionary_chunks,
                 info.chunk_stats.entropy_chunks);
    }
    
    Ok(())
}

fn validation_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n6. Validation and Error Handling Example");
    println!("----------------------------------------");
    
    let test_data = b"Data for validation testing";
    let config = HlcConfig::default();
    
    // Compress data
    let compressed = compress_data(test_data, &config)?;
    println!("Original compressed data is valid: {}", validate_data(&compressed)?);
    
    // Test with corrupted data
    let mut corrupted = compressed.clone();
    if !corrupted.is_empty() {
        let len = corrupted.len();
        corrupted[len - 1] ^= 0xFF; // Flip some bits
        
        match validate_data(&corrupted) {
            Ok(false) => println!("Corrupted data correctly detected as invalid"),
            Err(e) => println!("Corrupted data validation failed with error: {}", e),
            Ok(true) => println!("Warning: Corrupted data not detected!"),
        }
        
        match decompress_data(&corrupted) {
            Err(e) => println!("Decompression of corrupted data failed as expected: {}", e),
            Ok(_) => println!("Warning: Corrupted data decompressed successfully!"),
        }
    }
    
    // Test with invalid format
    let invalid_data = b"This is not HLC data";
    match decompress_data(invalid_data) {
        Err(e) => println!("Invalid format correctly rejected: {}", e),
        Ok(_) => println!("Warning: Invalid data was accepted!"),
    }
    
    Ok(())
}

fn advanced_features_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n7. Advanced Features Example");
    println!("----------------------------");
    
    // Version information
    let version_info = hlc::version_info();
    println!("HLC Version: {}", version_info.version);
    println!("Description: {}", version_info.description);
    
    // Create test data with specific patterns
    let mut complex_data = Vec::new();
    
    // Add structured data that should benefit from delta compression
    for i in 0..1000u32 {
        complex_data.extend(i.to_le_bytes());
    }
    
    // Add sparse data for RLE
    complex_data.extend(vec![0u8; 1000]);
    for i in (0..1000).step_by(100) {
        let base_idx = complex_data.len() - 1000;
        complex_data[base_idx + i] = 255;
    }
    
    // Add repeating patterns for dictionary compression
    complex_data.extend(b"REPEATING_PATTERN_".repeat(100));
    
    println!("Complex data size: {} bytes", complex_data.len());
    
    // Compress with maximum settings
    let config = HlcConfig::default()
        .with_mode(HlcMode::Max)
        .with_checksum(ChecksumType::SHA256)
        .with_threads(num_cpus::get())
        .with_chunk_size(16384);
    
    let start = std::time::Instant::now();
    let compressed = compress_data(&complex_data, &config)?;
    let compression_time = start.elapsed();
    
    let info = get_compression_info(&compressed)?;
    
    println!("Advanced compression results:");
    println!("  Compressed size: {} bytes", compressed.len());
    println!("  Compression ratio: {:.2}x", info.compression_ratio);
    println!("  Space saved: {:.1}%", info.chunk_stats.space_saved_percentage());
    println!("  Total chunks: {}", info.total_chunks);
    println!("  Compression time: {:?}", compression_time);
    
    // Detailed transform statistics
    println!("Transform usage:");
    println!("  Stored (no compression): {}/{}", info.chunk_stats.stored_chunks, info.total_chunks);
    println!("  RLE encoding: {}/{}", info.chunk_stats.rle_chunks, info.total_chunks);
    println!("  Delta encoding: {}/{}", info.chunk_stats.delta_chunks, info.total_chunks);
    println!("  Dictionary compression: {}/{}", info.chunk_stats.dictionary_chunks, info.total_chunks);
    println!("  Entropy coding: {}/{}", info.chunk_stats.entropy_chunks, info.total_chunks);
    
    // Verify decompression
    let start = std::time::Instant::now();
    let decompressed = decompress_data(&compressed)?;
    let decompression_time = start.elapsed();
    
    assert_eq!(complex_data, decompressed);
    println!("  Decompression time: {:?}", decompression_time);
    println!("✓ Advanced compression/decompression verified");
    
    Ok(())
}

/// Helper function to demonstrate error handling patterns
fn demonstrate_error_handling() -> Result<(), hlc::HlcError> {
    use hlc::HlcError;
    
    // This function shows how to handle different types of HLC errors
    let invalid_data = b"Not HLC data";
    
    match decompress_data(invalid_data) {
        Ok(_) => println!("Unexpected success"),
        Err(HlcError::InvalidFormat(msg)) => {
            println!("Invalid format detected: {}", msg);
            // Handle format error specifically
        },
        Err(HlcError::ChecksumMismatch) => {
            println!("Data corruption detected");
            // Handle corruption specifically
        },
        Err(HlcError::Io(io_err)) => {
            println!("I/O error: {}", io_err);
            // Handle I/O errors
        },
        Err(e) => {
            println!("Other error: {}", e);
            // Handle other error types
        }
    }
    
    Ok(())
}

/// Helper function to show performance measurement
fn measure_performance(data: &[u8], config: &HlcConfig) -> Result<(), Box<dyn std::error::Error>> {
    let iterations = 5;
    let mut total_compression_time = std::time::Duration::from_secs(0);
    let mut total_decompression_time = std::time::Duration::from_secs(0);
    let mut compressed_sizes = Vec::new();
    
    for _ in 0..iterations {
        // Measure compression
        let start = std::time::Instant::now();
        let compressed = compress_data(data, config)?;
        let compression_time = start.elapsed();
        total_compression_time += compression_time;
        compressed_sizes.push(compressed.len());
        
        // Measure decompression
        let start = std::time::Instant::now();
        let _decompressed = decompress_data(&compressed)?;
        let decompression_time = start.elapsed();
        total_decompression_time += decompression_time;
    }
    
    let avg_compression_time = total_compression_time / iterations as u32;
    let avg_decompression_time = total_decompression_time / iterations as u32;
    let avg_compressed_size = compressed_sizes.iter().sum::<usize>() / iterations;
    
    println!("Performance metrics ({} iterations):", iterations);
    println!("  Average compression time: {:?}", avg_compression_time);
    println!("  Average decompression time: {:?}", avg_decompression_time);
    println!("  Average compressed size: {} bytes", avg_compressed_size);
    println!("  Average compression ratio: {:.2}x", data.len() as f64 / avg_compressed_size as f64);
    
    let compression_throughput = (data.len() as f64 / (1024.0 * 1024.0)) / avg_compression_time.as_secs_f64();
    let decompression_throughput = (data.len() as f64 / (1024.0 * 1024.0)) / avg_decompression_time.as_secs_f64();
    
    println!("  Compression throughput: {:.2} MB/s", compression_throughput);
    println!("  Decompression throughput: {:.2} MB/s", decompression_throughput);
    
    Ok(())
}