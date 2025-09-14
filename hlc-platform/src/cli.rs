use crate::config::{HlcConfig, HlcMode};
use crate::error::HlcError;
use crate::pipeline;
use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[clap(author, version, about, long_about = "Hybrid Lossless Compression (HLC) Platform")]
struct Cli {
	#[clap(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Compress a file
	Compress {
		#[clap(short, long, value_name = "FILE")]
		input: PathBuf,
		#[clap(short, long, value_name = "FILE")]
		output: PathBuf,
		#[clap(short, long, default_value = "balanced")]
		mode: HlcMode,
		#[clap(short, long)]
		threads: Option<usize>,
	},
	/// Decompress a file
	Decompress {
		#[clap(short, long, value_name = "FILE")]
		input: PathBuf,
		#[clap(short, long, value_name = "FILE")]
		output: PathBuf,
		#[clap(short, long)]
		threads: Option<usize>,
	},
}

pub fn run() -> Result<(), HlcError> {
	let cli = Cli::parse();

	match &cli.command {
		Commands::Compress { input, output, mode, threads } => {
			println!("Compressing {} to {}...", input.display(), output.display());
			let config = HlcConfig {
				mode: *mode,
				threads: threads.unwrap_or_else(num_cpus::get),
				..Default::default()
			};
			let mut in_file = BufReader::new(File::open(input)?);
			let mut out_file = BufWriter::new(File::create(output)?);
			let start = Instant::now();
			let stats = pipeline::compress(&mut in_file, &mut out_file, &config)?;
			let duration = start.elapsed();
			println!("Compression successful!");
			println!("  Original Size:    {} bytes", stats.original_size);
			println!("  Compressed Size:  {} bytes", stats.compressed_size);
			println!("  Ratio:            {:.2}x", stats.ratio);
			println!("  Elapsed Time:     {:.2?}", duration);
		}
		Commands::Decompress { input, output, threads } => {
			println!("Decompressing {} to {}...", input.display(), output.display());
			let num_threads = threads.unwrap_or_else(num_cpus::get);
			let mut in_file = BufReader::new(File::open(input)?);
			let mut out_file = BufWriter::new(File::create(output)?);
			let start = Instant::now();
			pipeline::decompress(&mut in_file, &mut out_file, num_threads)?;
			let duration = start.elapsed();
			println!("Decompression successful!");
			println!("  Elapsed Time: {:.2?}", duration);
		}
	}
	Ok(())
}