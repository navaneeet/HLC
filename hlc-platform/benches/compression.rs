//! Comprehensive benchmarks for the HLC compression platform
//! 
//! These benchmarks measure performance across different data types,
//! compression modes, and system configurations.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use hlc::{HlcConfig, HlcMode, ChecksumType, compress_data, decompress_data};
use std::time::Duration;

/// Generate different types of test data for benchmarking
fn generate_test_data() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        // Highly compressible data (zeros)
        ("zeros_1kb", vec![0u8; 1024]),
        ("zeros_10kb", vec![0u8; 10 * 1024]),
        ("zeros_100kb", vec![0u8; 100 * 1024]),
        
        // Sequential data (good for delta compression)
        ("sequential_1kb", (0u8..255).cycle().take(1024).collect()),
        ("sequential_10kb", (0u8..255).cycle().take(10 * 1024).collect()),
        ("sequential_100kb", (0u8..255).cycle().take(100 * 1024).collect()),
        
        // Sparse data (good for RLE)
        ("sparse_1kb", {
            let mut data = vec![0u8; 1024];
            for i in (0..1024).step_by(64) {
                data[i] = 255;
            }
            data
        }),
        ("sparse_10kb", {
            let mut data = vec![0u8; 10 * 1024];
            for i in (0..10 * 1024).step_by(64) {
                data[i] = 255;
            }
            data
        }),
        ("sparse_100kb", {
            let mut data = vec![0u8; 100 * 1024];
            for i in (0..100 * 1024).step_by(64) {
                data[i] = 255;
            }
            data
        }),
        
        // Text-like data
        ("text_1kb", "The quick brown fox jumps over the lazy dog. ".repeat(21)[..1024].as_bytes().to_vec()),
        ("text_10kb", "The quick brown fox jumps over the lazy dog. ".repeat(227)[..10 * 1024].as_bytes().to_vec()),
        ("text_100kb", "The quick brown fox jumps over the lazy dog. ".repeat(2275)[..100 * 1024].as_bytes().to_vec()),
        
        // Random data (difficult to compress)
        ("random_1kb", (0..1024).map(|i| ((i * 17 + 42) % 256) as u8).collect()),
        ("random_10kb", (0..10 * 1024).map(|i| ((i * 17 + 42) % 256) as u8).collect()),
        ("random_100kb", (0..100 * 1024).map(|i| ((i * 17 + 42) % 256) as u8).collect()),
        
        // Binary data (integers)
        ("binary_1kb", (0u32..256).flat_map(|i| i.to_le_bytes()).collect()),
        ("binary_10kb", (0u32..2560).flat_map(|i| i.to_le_bytes()).collect()),
        ("binary_100kb", (0u32..25600).flat_map(|i| i.to_le_bytes()).collect()),
        
        // Repeated patterns
        ("patterns_1kb", b"ABCDEFGHIJKLMNOP".repeat(64)),
        ("patterns_10kb", b"ABCDEFGHIJKLMNOP".repeat(640)),
        ("patterns_100kb", b"ABCDEFGHIJKLMNOP".repeat(6400)),
    ]
}

/// Benchmark compression performance across different data types and modes
fn bench_compression_modes(c: &mut Criterion) {
    let test_data = generate_test_data();
    let modes = vec![
        ("balanced", HlcMode::Balanced),
        ("max", HlcMode::Max),
    ];
    
    let mut group = c.benchmark_group("compression_modes");
    
    for (data_name, data) in &test_data {
        group.throughput(Throughput::Bytes(data.len() as u64));
        
        for (mode_name, mode) in &modes {
            let config = HlcConfig::default().with_mode(*mode);
            let benchmark_name = format!("{}_{}", data_name, mode_name);
            
            group.bench_with_input(
                BenchmarkId::new("compress", &benchmark_name),
                data,
                |b, data| {
                    b.iter(|| {
                        let compressed = compress_data(black_box(data), black_box(&config)).unwrap();
                        black_box(compressed);
                    });
                },
            );
        }
    }
    
    group.finish();
}

/// Benchmark decompression performance
fn bench_decompression(c: &mut Criterion) {
    let test_data = generate_test_data();
    let config = HlcConfig::default();
    
    // Pre-compress all test data
    let compressed_data: Vec<_> = test_data
        .into_iter()
        .map(|(name, data)| {
            let compressed = compress_data(&data, &config).unwrap();
            (name, data.len(), compressed)
        })
        .collect();
    
    let mut group = c.benchmark_group("decompression");
    
    for (data_name, original_size, compressed) in &compressed_data {
        group.throughput(Throughput::Bytes(*original_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("decompress", data_name),
            compressed,
            |b, compressed| {
                b.iter(|| {
                    let decompressed = decompress_data(black_box(compressed)).unwrap();
                    black_box(decompressed);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark different thread counts
fn bench_thread_scaling(c: &mut Criterion) {
    let test_data = vec![0u8; 1024 * 1024]; // 1MB of compressible data
    let thread_counts = vec![1, 2, 4, 8, 16];
    
    let mut group = c.benchmark_group("thread_scaling");
    group.throughput(Throughput::Bytes(test_data.len() as u64));
    group.measurement_time(Duration::from_secs(10));
    
    for threads in thread_counts {
        let config = HlcConfig::default().with_threads(threads);
        
        group.bench_with_input(
            BenchmarkId::new("compress", threads),
            &test_data,
            |b, data| {
                b.iter(|| {
                    let compressed = compress_data(black_box(data), black_box(&config)).unwrap();
                    black_box(compressed);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark different chunk sizes
fn bench_chunk_sizes(c: &mut Criterion) {
    let test_data = vec![42u8; 1024 * 1024]; // 1MB of data
    let chunk_sizes = vec![1024, 4096, 16384, 65536, 262144, 1048576];
    
    let mut group = c.benchmark_group("chunk_sizes");
    group.throughput(Throughput::Bytes(test_data.len() as u64));
    group.measurement_time(Duration::from_secs(10));
    
    for chunk_size in chunk_sizes {
        let config = HlcConfig::default().with_chunk_size(chunk_size);
        
        group.bench_with_input(
            BenchmarkId::new("compress", chunk_size),
            &test_data,
            |b, data| {
                b.iter(|| {
                    let compressed = compress_data(black_box(data), black_box(&config)).unwrap();
                    black_box(compressed);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark different checksum types
fn bench_checksum_types(c: &mut Criterion) {
    let test_data = vec![123u8; 100 * 1024]; // 100KB
    let checksum_types = vec![
        ("crc32", ChecksumType::CRC32),
        ("sha256", ChecksumType::SHA256),
    ];
    
    let mut group = c.benchmark_group("checksum_types");
    group.throughput(Throughput::Bytes(test_data.len() as u64));
    
    for (name, checksum_type) in checksum_types {
        let config = HlcConfig::default().with_checksum(checksum_type);
        
        group.bench_with_input(
            BenchmarkId::new("compress", name),
            &test_data,
            |b, data| {
                b.iter(|| {
                    let compressed = compress_data(black_box(data), black_box(&config)).unwrap();
                    black_box(compressed);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark compression ratio vs speed tradeoffs
fn bench_compression_ratios(c: &mut Criterion) {
    let test_data_types = vec![
        ("compressible", vec![0u8; 100 * 1024]),
        ("incompressible", (0..100 * 1024).map(|i| ((i * 17 + 42) % 256) as u8).collect()),
    ];
    
    let mut group = c.benchmark_group("compression_ratios");
    
    for (data_type, data) in &test_data_types {
        group.throughput(Throughput::Bytes(data.len() as u64));
        
        let balanced_config = HlcConfig::default().with_mode(HlcMode::Balanced);
        let max_config = HlcConfig::default().with_mode(HlcMode::Max);
        
        // Measure compression performance
        group.bench_with_input(
            BenchmarkId::new("balanced", data_type),
            data,
            |b, data| {
                b.iter(|| {
                    let compressed = compress_data(black_box(data), black_box(&balanced_config)).unwrap();
                    black_box(compressed);
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("max", data_type),
            data,
            |b, data| {
                b.iter(|| {
                    let compressed = compress_data(black_box(data), black_box(&max_config)).unwrap();
                    black_box(compressed);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark roundtrip (compress + decompress) performance
fn bench_roundtrip(c: &mut Criterion) {
    let test_data = vec![42u8; 100 * 1024]; // 100KB
    let config = HlcConfig::default();
    
    let mut group = c.benchmark_group("roundtrip");
    group.throughput(Throughput::Bytes(test_data.len() as u64));
    group.measurement_time(Duration::from_secs(10));
    
    group.bench_function("compress_decompress", |b| {
        b.iter(|| {
            let compressed = compress_data(black_box(&test_data), black_box(&config)).unwrap();
            let decompressed = decompress_data(black_box(&compressed)).unwrap();
            black_box(decompressed);
        });
    });
    
    group.finish();
}

/// Benchmark memory usage patterns (indirectly through performance)
fn bench_memory_patterns(c: &mut Criterion) {
    let sizes = vec![
        ("small", 1024),
        ("medium", 100 * 1024),
        ("large", 1024 * 1024),
        ("xlarge", 10 * 1024 * 1024),
    ];
    
    let mut group = c.benchmark_group("memory_patterns");
    group.measurement_time(Duration::from_secs(15));
    
    for (size_name, size) in sizes {
        let test_data = vec![42u8; size];
        let config = HlcConfig::default();
        
        group.throughput(Throughput::Bytes(size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("compress", size_name),
            &test_data,
            |b, data| {
                b.iter(|| {
                    let compressed = compress_data(black_box(data), black_box(&config)).unwrap();
                    black_box(compressed);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark individual transform performance
fn bench_transforms(c: &mut Criterion) {
    use hlc::transforms::{rle, delta, entropy};
    
    let test_data_types = vec![
        ("sparse", {
            let mut data = vec![0u8; 10 * 1024];
            for i in (0..10 * 1024).step_by(100) {
                data[i] = 255;
            }
            data
        }),
        ("sequential", (0u8..255).cycle().take(10 * 1024).collect()),
        ("random", (0..10 * 1024).map(|i| ((i * 17 + 42) % 256) as u8).collect()),
    ];
    
    let mut group = c.benchmark_group("individual_transforms");
    
    for (data_name, data) in &test_data_types {
        group.throughput(Throughput::Bytes(data.len() as u64));
        
        // RLE encoding
        group.bench_with_input(
            BenchmarkId::new("rle_encode", data_name),
            data,
            |b, data| {
                b.iter(|| {
                    let encoded = rle::encode(black_box(data));
                    black_box(encoded);
                });
            },
        );
        
        // Delta encoding
        group.bench_with_input(
            BenchmarkId::new("delta_encode", data_name),
            data,
            |b, data| {
                b.iter(|| {
                    let encoded = delta::encode(black_box(data));
                    black_box(encoded);
                });
            },
        );
        
        // Entropy encoding
        group.bench_with_input(
            BenchmarkId::new("entropy_encode", data_name),
            data,
            |b, data| {
                b.iter(|| {
                    let encoded = entropy::encode(black_box(data), 5).unwrap();
                    black_box(encoded);
                });
            },
        );
    }
    
    group.finish();
}

/// Custom benchmark for real-world data patterns
fn bench_realistic_data(c: &mut Criterion) {
    // Simulate realistic data patterns
    let realistic_data = vec![
        ("log_data", {
            let mut data = Vec::new();
            let timestamp_base = 1234567890u32;
            let log_levels = [b"INFO", b"WARN", b"ERROR", b"DEBUG"];
            let messages = [
                b"User login successful",
                b"Database connection established",
                b"Cache miss for key",
                b"Request processed in 45ms",
                b"Memory usage: 67%",
            ];
            
            for i in 0..1000 {
                // Timestamp (sequential, good for delta)
                data.extend((timestamp_base + i).to_le_bytes());
                data.push(b' ');
                
                // Log level (repeated patterns)
                data.extend(log_levels[i % log_levels.len()]);
                data.push(b' ');
                
                // Message (some repetition)
                data.extend(messages[i % messages.len()]);
                data.push(b'\n');
            }
            data
        }),
        
        ("sensor_data", {
            let mut data = Vec::new();
            let mut temperature = 20.0f32;
            let mut humidity = 50.0f32;
            
            for _ in 0..5000 {
                // Gradually changing sensor values (good for delta)
                temperature += (rand::random::<f32>() - 0.5) * 0.1;
                humidity += (rand::random::<f32>() - 0.5) * 0.2;
                
                data.extend(temperature.to_le_bytes());
                data.extend(humidity.to_le_bytes());
                data.extend([0u8; 8]); // Padding (good for RLE)
            }
            data
        }),
        
        ("config_data", {
            let config_template = br#"{"server": {"host": "localhost", "port": 8080, "ssl": false}, "database": {"url": "postgres://localhost/db", "pool_size": 10}, "logging": {"level": "info", "file": "/var/log/app.log"}}"#;
            config_template.repeat(100)
        }),
    ];
    
    let mut group = c.benchmark_group("realistic_data");
    
    for (data_name, data) in &realistic_data {
        group.throughput(Throughput::Bytes(data.len() as u64));
        
        let config = HlcConfig::default();
        
        group.bench_with_input(
            BenchmarkId::new("compress", data_name),
            data,
            |b, data| {
                b.iter(|| {
                    let compressed = compress_data(black_box(data), black_box(&config)).unwrap();
                    black_box(compressed);
                });
            },
        );
    }
    
    group.finish();
}

// Define all benchmark groups
criterion_group!(
    benches,
    bench_compression_modes,
    bench_decompression,
    bench_thread_scaling,
    bench_chunk_sizes,
    bench_checksum_types,
    bench_compression_ratios,
    bench_roundtrip,
    bench_memory_patterns,
    bench_transforms,
    bench_realistic_data
);

criterion_main!(benches);