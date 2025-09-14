use crate::config::{HlcConfig, ChecksumType};
use crate::container::{CompressedChunk, PipelineFlags};
use crate::error::HlcError;
use crate::transforms::{analyzer, delta, entropy, rle};
use crc32fast::Hasher as Crc32Hasher;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct RawChunk {
	pub id: usize,
	pub data: Vec<u8>,
}

pub fn process_chunk(chunk: RawChunk, config: &HlcConfig) -> Result<CompressedChunk, HlcError> {
	let original_size = chunk.data.len();
	let checksum = calculate_checksum(&chunk.data, config.checksum);

	let strategy = analyzer::analyze_chunk(&chunk.data, config.mode);
	let (mut transformed_data, flags) = apply_transforms(chunk.data, &strategy);

	if transformed_data.len() >= original_size {
		transformed_data = strategy.original_data;
	}

	let compressed_data = entropy::encode(&transformed_data)?;

	if compressed_data.len() >= original_size {
		Ok(CompressedChunk {
			id: chunk.id,
			flags: PipelineFlags::STORED,
			original_checksum: checksum,
			original_size: original_size as u32,
			data: transformed_data,
		})
	} else {
		Ok(CompressedChunk {
			id: chunk.id,
			flags,
			original_checksum: checksum,
			original_size: original_size as u32,
			data: compressed_data,
		})
	}
}

fn apply_transforms(
	mut data: Vec<u8>,
	strategy: &analyzer::CompressionStrategy,
) -> (Vec<u8>, PipelineFlags) {
	let mut flags = PipelineFlags::ENTROPY;
	if strategy.use_rle { data = rle::encode(&data); flags |= PipelineFlags::RLE; }
	if strategy.use_delta { data = delta::encode(&data); flags |= PipelineFlags::DELTA; }
	(data, flags)
}

pub fn calculate_checksum(data: &[u8], checksum_type: ChecksumType) -> u64 {
	match checksum_type {
		ChecksumType::CRC32 => {
			let mut hasher = Crc32Hasher::new();
			hasher.update(data);
			hasher.finalize() as u64
		}
		ChecksumType::SHA256 => {
			let mut hasher = Sha256::new();
			hasher.update(data);
			let hash = hasher.finalize();
			u64::from_le_bytes(hash[0..8].try_into().unwrap())
		}
	}
}