use crate::config::{HlcConfig, HlcMode, ChecksumType};
use crate::error::HlcError;
use crate::pipeline;
use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[clap(
    name = "hlc",
    version = "0.1.0",
    author = "Project Vision Team",
    about = "Hybrid Lossless Compression (HLC) Platform",
    long_about = "A high-performance lossless compression tool that uses adaptive multi-stage compression pipeline with parallel processing."
)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[clap(short, long, global = true)]
    pub verbose: bool,

    /// Suppress all output except errors
    #[clap(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Compress a file using HLC
    Compress {
        /// Input file to compress
        #[clap(short, long, value_name = "FILE")]
        input: PathBuf,

        /// Output file name (.hlc extension will be added if not present)
        #[clap(short, long, value_name = "FILE")]
        output: PathBuf,

        /// Compression mode
        #[clap(short, long, default_value = "balanced")]
        mode: HlcMode,

        /// Checksum type for data integrity
        #[clap(long, default_value = "crc32")]
        checksum: ChecksumType,

        /// Number of threads to use (default: all available cores)
        #[clap(short, long)]
        threads: Option<usize>,

        /// Chunk size in bytes (default: 1MB)
        #[clap(long)]
        chunk_size: Option<usize>,

        /// Force overwrite output file if it exists
        #[clap(short, long)]
        force: bool,
    },

    /// Decompress an HLC file
    Decompress {
        /// Input HLC file to decompress
        #[clap(short, long, value_name = "FILE")]
        input: PathBuf,

        /// Output file name
        #[clap(short, long, value_name = "FILE")]
        output: PathBuf,

        /// Number of threads to use (default: all available cores)
        #[clap(short, long)]
        threads: Option<usize>,

        /// Force overwrite output file if it exists
        #[clap(short, long)]
        force: bool,
    },

    /// Display information about an HLC file
    Info {
        /// HLC file to analyze
        #[clap(value_name = "FILE")]
        input: PathBuf,
    },

    /// Validate the integrity of an HLC file
    Validate {
        /// HLC file to validate
        #[clap(value_name = "FILE")]
        input: PathBuf,
    },

    /// Estimate compression ratio for a file
    Estimate {
        /// File to analyze
        #[clap(value_name = "FILE")]
        input: PathBuf,

        /// Compression mode to use for estimation
        #[clap(short, long, default_value = "balanced")]
        mode: HlcMode,

        /// Chunk size for analysis
        #[clap(long)]
        chunk_size: Option<usize>,
    },

    /// Benchmark compression performance
    Benchmark {
        /// File to benchmark
        #[clap(value_name = "FILE")]
        input: PathBuf,

        /// Run multiple iterations
        #[clap(short, long, default_value = "3")]
        iterations: usize,

        /// Test both compression modes
        #[clap(long)]
        all_modes: bool,
    },
}

pub fn run() -> Result<(), HlcError> {
    let cli = Cli::parse();

    // Set up logging based on verbosity
    if !cli.quiet {
        let log_level = if cli.verbose { "debug" } else { "info" };
        let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level))
            .try_init(); // Use try_init to avoid panicking if already initialized
    }

    match cli.command {
        Commands::Compress {
            input,
            output,
            mode,
            checksum,
            threads,
            chunk_size,
            force,
        } => {
            compress_command(input, output, mode, checksum, threads, chunk_size, force, cli.quiet)
        }
        Commands::Decompress {
            input,
            output,
            threads,
            force,
        } => decompress_command(input, output, threads, force, cli.quiet),
        Commands::Info { input } => info_command(input),
        Commands::Validate { input } => validate_command(input, cli.quiet),
        Commands::Estimate {
            input,
            mode,
            chunk_size,
        } => estimate_command(input, mode, chunk_size),
        Commands::Benchmark {
            input,
            iterations,
            all_modes,
        } => benchmark_command(input, iterations, all_modes),
    }
}

fn compress_command(
    input: PathBuf,
    output: PathBuf,
    mode: HlcMode,
    checksum: ChecksumType,
    threads: Option<usize>,
    chunk_size: Option<usize>,
    force: bool,
    quiet: bool,
) -> Result<(), HlcError> {
    // Validate input file
    if !input.exists() {
        return Err(HlcError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Input file '{}' not found", input.display()),
        )));
    }

    // Check if output file exists
    if output.exists() && !force {
        return Err(HlcError::ConfigError(
            format!("Output file '{}' already exists. Use --force to overwrite.", output.display())
        ));
    }

    // Build configuration
    let mut config = HlcConfig::new()
        .with_mode(mode)
        .with_checksum(checksum);

    if let Some(t) = threads {
        config = config.with_threads(t);
    }

    if let Some(cs) = chunk_size {
        config = config.with_chunk_size(cs);
    }

    if !quiet {
        println!("Compressing '{}' to '{}'...", input.display(), output.display());
        println!("Configuration:");
        println!("  Mode: {:?}", config.mode);
        println!("  Checksum: {:?}", config.checksum);
        println!("  Threads: {}", config.threads);
        println!("  Chunk size: {} bytes", config.chunk_size);
    }

    let start = Instant::now();

    // Open files
    let input_file = File::open(&input)?;
    let mut reader = BufReader::new(input_file);
    
    let output_file = File::create(&output)?;
    let mut writer = BufWriter::new(output_file);

    // Perform compression
    let stats = pipeline::compress(&mut reader, &mut writer, &config)?;
    let duration = start.elapsed();

    if !quiet {
        println!("\nCompression completed successfully!");
        println!("  Original size:    {} bytes ({:.2} MB)", 
                 stats.original_size, 
                 stats.original_size as f64 / (1024.0 * 1024.0));
        println!("  Compressed size:  {} bytes ({:.2} MB)", 
                 stats.compressed_size, 
                 stats.compressed_size as f64 / (1024.0 * 1024.0));
        println!("  Compression ratio: {:.2}x", stats.ratio);
        println!("  Space saved:      {:.1}%", stats.space_saved_percentage());
        println!("  Chunks processed: {}", stats.chunks_processed);
        println!("  Processing time:  {:.2?}", duration);
        println!("  Throughput:       {:.2} MB/s", stats.throughput_mbps());

        println!("\nTransform statistics:");
        println!("  Stored chunks:    {}", stats.chunk_stats.stored_chunks);
        println!("  RLE encoded:      {}", stats.chunk_stats.rle_chunks);
        println!("  Delta encoded:    {}", stats.chunk_stats.delta_chunks);
        println!("  Dictionary:       {}", stats.chunk_stats.dictionary_chunks);
        println!("  Entropy coded:    {}", stats.chunk_stats.entropy_chunks);
    }

    Ok(())
}

fn decompress_command(
    input: PathBuf,
    output: PathBuf,
    threads: Option<usize>,
    force: bool,
    quiet: bool,
) -> Result<(), HlcError> {
    // Validate input file
    if !input.exists() {
        return Err(HlcError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Input file '{}' not found", input.display()),
        )));
    }

    // Check if output file exists
    if output.exists() && !force {
        return Err(HlcError::ConfigError(
            format!("Output file '{}' already exists. Use --force to overwrite.", output.display())
        ));
    }

    let num_threads = threads.unwrap_or_else(num_cpus::get);

    if !quiet {
        println!("Decompressing '{}' to '{}'...", input.display(), output.display());
        println!("Using {} threads", num_threads);
    }

    let start = Instant::now();

    // Open files
    let input_file = File::open(&input)?;
    let mut reader = BufReader::new(input_file);
    
    let output_file = File::create(&output)?;
    let mut writer = BufWriter::new(output_file);

    // Perform decompression
    pipeline::decompress(&mut reader, &mut writer, num_threads)?;
    let duration = start.elapsed();

    if !quiet {
        println!("Decompression completed successfully!");
        println!("  Processing time: {:.2?}", duration);
    }

    Ok(())
}

fn info_command(input: PathBuf) -> Result<(), HlcError> {
    if !input.exists() {
        return Err(HlcError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Input file '{}' not found", input.display()),
        )));
    }

    let input_file = File::open(&input)?;
    let mut reader = BufReader::new(input_file);

    let info = pipeline::info(&mut reader)?;
    info.print_summary();

    Ok(())
}

fn validate_command(input: PathBuf, quiet: bool) -> Result<(), HlcError> {
    if !input.exists() {
        return Err(HlcError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Input file '{}' not found", input.display()),
        )));
    }

    if !quiet {
        println!("Validating '{}'...", input.display());
    }

    let input_file = File::open(&input)?;
    let mut reader = BufReader::new(input_file);

    let start = Instant::now();
    let is_valid = pipeline::validate(&mut reader)?;
    let duration = start.elapsed();

    if is_valid {
        if !quiet {
            println!("✓ File is valid and can be decompressed successfully");
            println!("  Validation time: {:.2?}", duration);
        }
    } else {
        println!("✗ File validation failed");
        return Err(HlcError::InvalidFormat("File validation failed".to_string()));
    }

    Ok(())
}

fn estimate_command(
    input: PathBuf,
    mode: HlcMode,
    chunk_size: Option<usize>,
) -> Result<(), HlcError> {
    if !input.exists() {
        return Err(HlcError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Input file '{}' not found", input.display()),
        )));
    }

    let mut config = HlcConfig::new().with_mode(mode);
    if let Some(cs) = chunk_size {
        config = config.with_chunk_size(cs);
    }

    println!("Estimating compression ratio for '{}'...", input.display());
    println!("Mode: {:?}, Chunk size: {} bytes", config.mode, config.chunk_size);

    let input_file = File::open(&input)?;
    let mut reader = BufReader::new(input_file);

    let start = Instant::now();
    let estimated_ratio = pipeline::estimate_compression(&mut reader, &config)?;
    let duration = start.elapsed();

    println!("Estimated compression ratio: {:.2}x", estimated_ratio);
    println!("Estimation time: {:.2?}", duration);

    Ok(())
}

fn benchmark_command(
    input: PathBuf,
    iterations: usize,
    all_modes: bool,
) -> Result<(), HlcError> {
    if !input.exists() {
        return Err(HlcError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Input file '{}' not found", input.display()),
        )));
    }

    println!("Benchmarking compression performance for '{}'", input.display());
    println!("Iterations: {}", iterations);

    let modes = if all_modes {
        vec![HlcMode::Balanced, HlcMode::Max]
    } else {
        vec![HlcMode::Balanced]
    };

    for mode in modes {
        println!("\n--- {:?} Mode ---", mode);
        
        let mut total_duration = std::time::Duration::from_secs(0);
        let mut total_ratio = 0.0;
        let mut total_throughput = 0.0;

        for i in 1..=iterations {
            let config = HlcConfig::new().with_mode(mode);
            
            let input_file = File::open(&input)?;
            let mut reader = BufReader::new(input_file);
            
            let mut compressed_data = Vec::new();
            
            let start = Instant::now();
            let stats = pipeline::compress(&mut reader, &mut compressed_data, &config)?;
            let duration = start.elapsed();
            
            total_duration += duration;
            total_ratio += stats.ratio;
            total_throughput += stats.throughput_mbps();
            
            println!("  Iteration {}: {:.2}x ratio, {:.2} MB/s, {:?}", 
                     i, stats.ratio, stats.throughput_mbps(), duration);
        }

        let avg_duration = total_duration / iterations as u32;
        let avg_ratio = total_ratio / iterations as f64;
        let avg_throughput = total_throughput / iterations as f64;

        println!("  Average: {:.2}x ratio, {:.2} MB/s, {:?}", 
                 avg_ratio, avg_throughput, avg_duration);
    }

    Ok(())
}

/// Helper function to ensure output file has .hlc extension
pub fn ensure_hlc_extension(path: PathBuf) -> PathBuf {
    if path.extension().and_then(|s| s.to_str()) != Some("hlc") {
        path.with_extension("hlc")
    } else {
        path
    }
}

/// Helper function to format file sizes
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_ensure_hlc_extension() {
        let path1 = PathBuf::from("test.txt");
        let path2 = PathBuf::from("test.hlc");
        
        assert_eq!(ensure_hlc_extension(path1), PathBuf::from("test.hlc"));
        assert_eq!(ensure_hlc_extension(path2), PathBuf::from("test.hlc"));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
    }

    #[test]
    fn test_compress_decompress_cli() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary input file
        let mut input_file = NamedTempFile::new()?;
        let test_data = b"Hello, world! This is test data for CLI testing.";
        input_file.write_all(test_data)?;
        
        // Create temporary output files
        let compressed_file = NamedTempFile::new()?;
        let decompressed_file = NamedTempFile::new()?;

        // Test compression
        let result = compress_command(
            input_file.path().to_path_buf(),
            compressed_file.path().to_path_buf(),
            HlcMode::Balanced,
            ChecksumType::CRC32,
            Some(1),
            None,
            true,
            true, // quiet mode for test
        );
        assert!(result.is_ok());

        // Test decompression
        let result = decompress_command(
            compressed_file.path().to_path_buf(),
            decompressed_file.path().to_path_buf(),
            Some(1),
            true,
            true, // quiet mode for test
        );
        assert!(result.is_ok());

        // Verify the decompressed data matches original
        let decompressed_data = std::fs::read(decompressed_file.path())?;
        assert_eq!(test_data.to_vec(), decompressed_data);

        Ok(())
    }
}