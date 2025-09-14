//! Benchmark suite for HLC compression

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use hlc_rust::{HLCCompressor, Config, CompressionMode};
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn create_test_data(size_mb: usize) -> Vec<u8> {
    let mut data = Vec::new();
    let chunk = b"This is test data for compression benchmarking. ".repeat(100);
    let chunks_needed = (size_mb * 1024 * 1024) / chunk.len();
    
    for _ in 0..chunks_needed {
        data.extend_from_slice(&chunk);
    }
    
    // Pad to exact size
    let target_size = size_mb * 1024 * 1024;
    while data.len() < target_size {
        data.push(b'A');
    }
    
    data.truncate(target_size);
    data
}

fn create_json_data(size_mb: usize) -> Vec<u8> {
    let json_template = r#"{
        "id": {},
        "name": "Test Item {}",
        "value": {},
        "items": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        "metadata": {
            "created": "2024-01-01T00:00:00Z",
            "updated": "2024-01-01T00:00:00Z",
            "tags": ["test", "benchmark", "compression"]
        }
    }"#;
    
    let mut data = Vec::new();
    let mut id = 0;
    
    while data.len() < size_mb * 1024 * 1024 {
        let json_item = json_template
            .replace("{}", &id.to_string())
            .replace("{}", &format!("Item-{}", id))
            .replace("{}", &(id * 42).to_string());
        
        data.extend_from_slice(json_item.as_bytes());
        data.push(b'\n');
        id += 1;
    }
    
    data.truncate(size_mb * 1024 * 1024);
    data
}

fn benchmark_compression_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_modes");
    
    let test_data = create_test_data(1); // 1MB
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.txt");
    let mut input_file = File::create(&input_path).unwrap();
    input_file.write_all(&test_data).unwrap();
    
    let modes = [CompressionMode::Fast, CompressionMode::Balanced, CompressionMode::Max];
    
    for mode in modes {
        let output_path = temp_dir.path().join(format!("output_{:?}.hlc", mode));
        let config = Config {
            mode,
            ..Default::default()
        };
        let compressor = HLCCompressor::new(config);
        
        group.bench_with_input(
            BenchmarkId::new("compress", format!("{:?}", mode)),
            &(input_path.clone(), output_path.clone()),
            |b, (input, output)| {
                b.to_async(tokio::runtime::Runtime::new().unwrap())
                    .iter(|| async {
                        compressor.compress_file(input, output).await.unwrap()
                    })
            },
        );
    }
    
    group.finish();
}

fn benchmark_different_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_sizes");
    
    let sizes = [1, 5, 10]; // MB
    let temp_dir = TempDir::new().unwrap();
    
    for size in sizes {
        let test_data = create_test_data(size);
        let input_path = temp_dir.path().join(format!("input_{}mb.txt", size));
        let output_path = temp_dir.path().join(format!("output_{}mb.hlc", size));
        
        let mut input_file = File::create(&input_path).unwrap();
        input_file.write_all(&test_data).unwrap();
        
        let config = Config::default();
        let compressor = HLCCompressor::new(config);
        
        group.bench_with_input(
            BenchmarkId::new("compress", format!("{}MB", size)),
            &(input_path.clone(), output_path.clone()),
            |b, (input, output)| {
                b.to_async(tokio::runtime::Runtime::new().unwrap())
                    .iter(|| async {
                        compressor.compress_file(input, output).await.unwrap()
                    })
            },
        );
    }
    
    group.finish();
}

fn benchmark_different_data_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_types");
    
    let temp_dir = TempDir::new().unwrap();
    let config = Config::default();
    let compressor = HLCCompressor::new(config);
    
    // Text data
    let text_data = create_test_data(1);
    let text_input = temp_dir.path().join("text_input.txt");
    let text_output = temp_dir.path().join("text_output.hlc");
    let mut text_file = File::create(&text_input).unwrap();
    text_file.write_all(&text_data).unwrap();
    
    group.bench_with_input(
        BenchmarkId::new("compress", "text"),
        &(text_input.clone(), text_output.clone()),
        |b, (input, output)| {
            b.to_async(tokio::runtime::Runtime::new().unwrap())
                .iter(|| async {
                    compressor.compress_file(input, output).await.unwrap()
                })
        },
    );
    
    // JSON data
    let json_data = create_json_data(1);
    let json_input = temp_dir.path().join("json_input.json");
    let json_output = temp_dir.path().join("json_output.hlc");
    let mut json_file = File::create(&json_input).unwrap();
    json_file.write_all(&json_data).unwrap();
    
    group.bench_with_input(
        BenchmarkId::new("compress", "json"),
        &(json_input.clone(), json_output.clone()),
        |b, (input, output)| {
            b.to_async(tokio::runtime::Runtime::new().unwrap())
                .iter(|| async {
                    compressor.compress_file(input, output).await.unwrap()
                })
        },
    );
    
    // Binary data
    let binary_data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
    let binary_input = temp_dir.path().join("binary_input.bin");
    let binary_output = temp_dir.path().join("binary_output.hlc");
    let mut binary_file = File::create(&binary_input).unwrap();
    binary_file.write_all(&binary_data).unwrap();
    
    group.bench_with_input(
        BenchmarkId::new("compress", "binary"),
        &(binary_input.clone(), binary_output.clone()),
        |b, (input, output)| {
            b.to_async(tokio::runtime::Runtime::new().unwrap())
                .iter(|| async {
                    compressor.compress_file(input, output).await.unwrap()
                })
        },
    );
    
    group.finish();
}

fn benchmark_thread_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread_scaling");
    
    let test_data = create_test_data(5); // 5MB
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.txt");
    let mut input_file = File::create(&input_path).unwrap();
    input_file.write_all(&test_data).unwrap();
    
    let thread_counts = [1, 2, 4, 8];
    
    for threads in thread_counts {
        let output_path = temp_dir.path().join(format!("output_{}threads.hlc", threads));
        let config = Config {
            threads,
            ..Default::default()
        };
        let compressor = HLCCompressor::new(config);
        
        group.bench_with_input(
            BenchmarkId::new("compress", format!("{}threads", threads)),
            &(input_path.clone(), output_path.clone()),
            |b, (input, output)| {
                b.to_async(tokio::runtime::Runtime::new().unwrap())
                    .iter(|| async {
                        compressor.compress_file(input, output).await.unwrap()
                    })
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_compression_modes,
    benchmark_different_sizes,
    benchmark_different_data_types,
    benchmark_thread_scaling
);
criterion_main!(benches);