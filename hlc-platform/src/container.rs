use crate::config::{ChecksumType, HlcConfig};
use crate::chunk::{RawChunk, calculate_checksum};
use crate::error::HlcError;
use crate::transforms::{delta, entropy, rle};
use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};
use std::io::{Read, Write};

const MAGIC_NUMBER: &[u8; 4] = b"HLC1";
const VERSION: u8 = 1;

bitflags::bitflags! {
	#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
	pub struct PipelineFlags: u8 {
		const STORED  = 0b00000001;
		const ENTROPY = 0b00000010;
		const RLE     = 0b00000100;
		const DELTA   = 0b00001000;
	}
}

#[derive(Debug)]
pub struct CompressedChunk {
	pub id: usize,
	pub flags: PipelineFlags,
	pub original_checksum: u64,
	pub original_size: u32,
	pub data: Vec<u8>,
}

impl CompressedChunk {
	pub fn decompress(&self) -> Result<RawChunk, HlcError> {
		let mut data = self.data.clone();

		if self.flags.contains(PipelineFlags::STORED) {
			// raw
		} else if self.flags.contains(PipelineFlags::ENTROPY) {
			data = entropy::decode(&data)?;
		} else {
			return Err(HlcError::DecompressionError("Unknown compression format".to_string()));
		}

		if self.flags.contains(PipelineFlags::DELTA) { data = delta::decode(&data); }
		if self.flags.contains(PipelineFlags::RLE) { data = rle::decode(&data); }

		if data.len() != self.original_size as usize {
			return Err(HlcError::DecompressionError("Decompressed size does not match original size".to_string()));
		}

		let checksum = calculate_checksum(&data, ChecksumType::CRC32);
		if checksum != self.original_checksum { return Err(HlcError::ChecksumMismatch); }

		Ok(RawChunk { id: self.id, data })
	}
}

pub fn write_hlc_container<W: Write>(
	writer: &mut W,
	chunks: &[CompressedChunk],
	config: &HlcConfig,
) -> Result<u64, HlcError> {
	let mut total_bytes_written = 0;

	writer.write_all(MAGIC_NUMBER)?;
	writer.write_u8(VERSION)?;
	let checksum_id = match config.checksum { ChecksumType::CRC32 => 0, ChecksumType::SHA256 => 1 };
	writer.write_u8(checksum_id)?;
	total_bytes_written += 6;

	for chunk in chunks {
		writer.write_u8(chunk.flags.bits())?;
		writer.write_u32::<LittleEndian>(chunk.original_size)?;
		writer.write_u32::<LittleEndian>(chunk.data.len() as u32)?;
		writer.write_u64::<LittleEndian>(chunk.original_checksum)?;
		writer.write_all(&chunk.data)?;
		total_bytes_written += (1 + 4 + 4 + 8) + chunk.data.len() as u64;
	}
	Ok(total_bytes_written)
}

pub fn read_hlc_container<R: Read>(reader: &mut R) -> Result<(Vec<CompressedChunk>, HlcConfig), HlcError> {
	let mut magic = [0u8; 4];
	reader.read_exact(&mut magic)?;
	if magic != *MAGIC_NUMBER { return Err(HlcError::InvalidFormat("Invalid magic number".to_string())); }

	let version = reader.read_u8()?;
	if version != VERSION { return Err(HlcError::InvalidFormat(format!("Unsupported version: {}", version))); }

	let checksum_id = reader.read_u8()?;
	let checksum = match checksum_id { 0 => ChecksumType::CRC32, 1 => ChecksumType::SHA256, _ => return Err(HlcError::InvalidFormat("Unknown checksum type".to_string())), };

	let config = HlcConfig { checksum, ..Default::default() };
	let mut chunks = Vec::new();
	let mut id_counter = 0;

	loop {
		match reader.read_u8() {
			Ok(flags_byte) => {
				let flags = PipelineFlags::from_bits_truncate(flags_byte);
				let original_size = reader.read_u32::<LittleEndian>()?;
				let compressed_size = reader.read_u32::<LittleEndian>()?;
				let original_checksum = reader.read_u64::<LittleEndian>()?;
				let mut data = vec![0; compressed_size as usize];
				reader.read_exact(&mut data)?;
				chunks.push(CompressedChunk { id: id_counter, flags, original_checksum, original_size, data });
				id_counter += 1;
			}
			Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => { break; }
			Err(e) => return Err(e.into()),
		}
	}
	Ok((chunks, config))
}