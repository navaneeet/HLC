use clap::{Parser, Subcommand};
use hlc_rust::{CompressionMode, Config, HLCCompressor};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "hlc")]
#[command(about = "High-Level Compression system with adaptive chunking")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compress a file or directory
    Compress {
        /// Input file or directory
        input: PathBuf,
        /// Output file or directory
        output: PathBuf,
        /// Compression mode
        #[arg(short, long, default_value = "balanced")]
        mode: String,
        /// Number of threads to use
        #[arg(short, long)]
        threads: Option<usize>,
        /// Enable SHA-256 checksums
        #[arg(long)]
        sha256: bool,
        /// Enable encryption
        #[arg(long)]
        encrypt: bool,
        /// Minimum chunk size
        #[arg(long, default_value = "1024")]
        chunk_min: usize,
        /// Maximum chunk size
        #[arg(long, default_value = "65536")]
        chunk_max: usize,
        /// Show progress
        #[arg(short, long)]
        progress: bool,
        /// Output statistics in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Decompress a file or directory
    Decompress {
        /// Input file or directory
        input: PathBuf,
        /// Output file or directory
        output: PathBuf,
        /// Show progress
        #[arg(short, long)]
        progress: bool,
    },
    /// Benchmark compression against other algorithms
    Benchmark {
        /// Input file for benchmarking
        input: PathBuf,
        /// Output directory for results
        output: Option<PathBuf>,
    },
}

fn parse_mode(mode_str: &str) -> anyhow::Result<CompressionMode> {
    match mode_str.to_lowercase().as_str() {
        "fast" => Ok(CompressionMode::Fast),
        "balanced" => Ok(CompressionMode::Balanced),
        "max" => Ok(CompressionMode::Max),
        _ => Err(anyhow::anyhow!("Invalid mode: {}. Must be one of: fast, balanced, max", mode_str)),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    let cli = Cli::parse();

    match cli.command {
        Commands::Compress {
            input,
            output,
            mode,
            threads,
            sha256,
            encrypt,
            chunk_min,
            chunk_max,
            progress,
            json,
        } => {
            let compression_mode = parse_mode(&mode)?;
            let config = Config {
                mode: compression_mode,
                threads: threads.unwrap_or_else(num_cpus::get),
                enable_sha256: sha256,
                enable_encryption: encrypt,
                chunk_size_min: chunk_min,
                chunk_size_max: chunk_max,
            };

            let compressor = HLCCompressor::new(config);
            
            if progress {
                println!("Compressing {} -> {}", input.display(), output.display());
            }

            let stats = compressor.compress_file(&input, &output).await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&stats)?);
            } else {
                println!("Compression complete!");
                println!("Original size: {} bytes", stats.original_size);
                println!("Compressed size: {} bytes", stats.compressed_size);
                println!("Compression ratio: {:.2}%", stats.compression_ratio * 100.0);
                println!("Compression time: {} ms", stats.compression_time_ms);
                println!("Chunks processed: {}", stats.chunks_processed);
            }
        }
        Commands::Decompress { input, output, progress } => {
            let config = Config::default();
            let compressor = HLCCompressor::new(config);
            
            if progress {
                println!("Decompressing {} -> {}", input.display(), output.display());
            }

            compressor.decompress_file(&input, &output).await?;
            println!("Decompression complete!");
        }
        Commands::Benchmark { input, output } => {
            println!("Running benchmarks on {}", input.display());
            // TODO: Implement benchmarking
            todo!("Benchmark implementation")
        }
    }

    Ok(())
}