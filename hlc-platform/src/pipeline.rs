use crate::chunk::{process_chunk, RawChunk};
use crate::config::HlcConfig;
use crate::container::{read_hlc_container, write_hlc_container, CompressedChunk};
use crate::error::HlcError;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::io::{Read, Write};

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MiB

#[derive(Debug)]
pub struct CompressionStats {
	pub original_size: u64,
	pub compressed_size: u64,
	pub ratio: f64,
}

pub fn compress<R: Read, W: Write>(
	reader: &mut R,
	writer: &mut W,
	config: &HlcConfig,
) -> Result<CompressionStats, HlcError> {
	let mut buffer = Vec::new();
	reader.read_to_end(&mut buffer)?;
	let original_size = buffer.len() as u64;

	let pb = ProgressBar::new(original_size);
	pb.set_style(ProgressStyle::default_bar()
		.template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
		.unwrap()
		.progress_chars("#>-")
	);

	let raw_chunks: Vec<RawChunk> = buffer
		.chunks(CHUNK_SIZE)
		.enumerate()
		.map(|(i, data)| RawChunk { id: i, data: data.to_vec() })
		.collect();

	let _ = rayon::ThreadPoolBuilder::new()
		.num_threads(config.threads)
		.build_global();

	let mut compressed_chunks: Vec<CompressedChunk> = raw_chunks
		.into_par_iter()
		.map(|chunk| {
			let len = chunk.data.len() as u64;
			let processed_chunk = process_chunk(chunk, config);
			pb.inc(len);
			processed_chunk
		})
		.collect::<Result<Vec<_>, HlcError>>()?;

	pb.finish_with_message("Compression finished");

	compressed_chunks.sort_by_key(|c| c.id);

	let compressed_size = write_hlc_container(writer, &compressed_chunks, config)?;
	let ratio = if compressed_size > 0 { original_size as f64 / compressed_size as f64 } else { 0.0 };

	Ok(CompressionStats { original_size, compressed_size, ratio })
}

pub fn decompress<R: Read, W: Write>(
	reader: &mut R,
	writer: &mut W,
	num_threads: usize,
) -> Result<(), HlcError> {
	let (chunks, _config) = read_hlc_container(reader)?;

	let _ = rayon::ThreadPoolBuilder::new()
		.num_threads(num_threads)
		.build_global();

	let pb = ProgressBar::new(chunks.len() as u64);
	pb.set_style(ProgressStyle::default_bar()
		.template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] Chunks {pos}/{len} ({eta})")
		.unwrap()
		.progress_chars("#>-")
	);

	let mut decoded_chunks: Vec<RawChunk> = chunks
		.into_par_iter()
		.map(|chunk| {
			let decoded_chunk = chunk.decompress()?;
			pb.inc(1);
			Ok::<RawChunk, HlcError>(decoded_chunk)
		})
		.collect::<Result<Vec<_>, HlcError>>()?;

	pb.finish_with_message("Decompression finished");

	decoded_chunks.sort_by_key(|c| c.id);
	for chunk in decoded_chunks { writer.write_all(&chunk.data)?; }
	Ok(())
}