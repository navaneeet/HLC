use crate::config::{ChecksumType, HlcConfig};
use crate::chunk::RawChunk;
use crate::error::HlcError;
use crate::transforms::{delta, entropy, rle, dictionary};
use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};
use crc32fast::Hasher as Crc32Hasher;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};

const MAGIC_NUMBER: &[u8; 4] = b"HLC1";
const VERSION: u8 = 1;

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct PipelineFlags: u8 {
        const STORED      = 0b00000001; // Data is stored uncompressed
        const ENTROPY     = 0b00000010; // Entropy coded (zstd)
        const RLE         = 0b00000100; // Run-Length Encoded
        const DELTA       = 0b00001000; // Delta coded
        const DICTIONARY  = 0b00010000; // Dictionary compressed
        // Reserved flags for future use
        const RESERVED_1  = 0b00100000;
        const RESERVED_2  = 0b01000000;
        const RESERVED_3  = 0b10000000;
    }
}

#[derive(Debug, Clone)]
pub struct CompressedChunk {
    pub id: usize,
    pub flags: PipelineFlags,
    pub original_checksum: u64,
    pub original_size: u32,
    pub compressed_size: u32,
    pub data: Vec<u8>,
}

impl CompressedChunk {
    pub fn new(id: usize, data: Vec<u8>, original_size: usize, checksum: u64) -> Self {
        Self {
            id,
            flags: PipelineFlags::STORED,
            original_checksum: checksum,
            original_size: original_size as u32,
            compressed_size: data.len() as u32,
            data,
        }
    }

    pub fn decompress(&self, config: &HlcConfig) -> Result<RawChunk, HlcError> {
        let mut data = self.data.clone();

        // Apply decompression in reverse order of compression
        if self.flags.contains(PipelineFlags::STORED) {
            // Data is stored raw, no processing needed
        } else {
            // First, decode entropy if applied
            if self.flags.contains(PipelineFlags::ENTROPY) {
                data = entropy::decode(&data)?;
            }

            // Then apply reverse transforms in reverse order
            if self.flags.contains(PipelineFlags::DICTIONARY) {
                data = dictionary::decode(&data);
            }
            
            if self.flags.contains(PipelineFlags::DELTA) {
                data = delta::decode(&data);
            }
            
            if self.flags.contains(PipelineFlags::RLE) {
                data = rle::decode(&data);
            }
        }

        // Verify size
        if data.len() != self.original_size as usize {
            return Err(HlcError::DecompressionError(format!(
                "Decompressed size {} does not match expected size {}",
                data.len(),
                self.original_size
            )));
        }

        // Verify checksum
        let checksum = calculate_checksum(&data, config.checksum);
        if checksum != self.original_checksum {
            return Err(HlcError::ChecksumMismatch);
        }

        Ok(RawChunk { id: self.id, data })
    }

    pub fn compression_ratio(&self) -> f64 {
        if self.compressed_size == 0 {
            return 0.0;
        }
        self.original_size as f64 / self.compressed_size as f64
    }
}

/// Container header structure
#[derive(Debug)]
pub struct ContainerHeader {
    pub version: u8,
    pub checksum_type: ChecksumType,
    pub chunk_count: u32,
    pub original_size: u64,
    pub compressed_size: u64,
    pub flags: u32, // Reserved for future container-level flags
}

impl ContainerHeader {
    pub fn new(checksum_type: ChecksumType, chunk_count: usize) -> Self {
        Self {
            version: VERSION,
            checksum_type,
            chunk_count: chunk_count as u32,
            original_size: 0,
            compressed_size: 0,
            flags: 0,
        }
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> Result<(), HlcError> {
        writer.write_all(MAGIC_NUMBER)?;
        writer.write_u8(self.version)?;
        
        let checksum_id = match self.checksum_type {
            ChecksumType::CRC32 => 0,
            ChecksumType::SHA256 => 1,
        };
        writer.write_u8(checksum_id)?;
        
        writer.write_u32::<LittleEndian>(self.chunk_count)?;
        writer.write_u64::<LittleEndian>(self.original_size)?;
        writer.write_u64::<LittleEndian>(self.compressed_size)?;
        writer.write_u32::<LittleEndian>(self.flags)?;
        
        Ok(())
    }

    pub fn read<R: Read>(reader: &mut R) -> Result<Self, HlcError> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if magic != *MAGIC_NUMBER {
            return Err(HlcError::InvalidFormat("Invalid magic number".to_string()));
        }

        let version = reader.read_u8()?;
        if version != VERSION {
            return Err(HlcError::InvalidFormat(format!("Unsupported version: {}", version)));
        }

        let checksum_id = reader.read_u8()?;
        let checksum_type = match checksum_id {
            0 => ChecksumType::CRC32,
            1 => ChecksumType::SHA256,
            _ => return Err(HlcError::InvalidFormat("Unknown checksum type".to_string())),
        };

        let chunk_count = reader.read_u32::<LittleEndian>()?;
        let original_size = reader.read_u64::<LittleEndian>()?;
        let compressed_size = reader.read_u64::<LittleEndian>()?;
        let flags = reader.read_u32::<LittleEndian>()?;

        Ok(Self {
            version,
            checksum_type,
            chunk_count,
            original_size,
            compressed_size,
            flags,
        })
    }

    pub fn size() -> usize {
        4 + 1 + 1 + 4 + 8 + 8 + 4 // magic + version + checksum + chunk_count + sizes + flags
    }
}

/// Writes the complete HLC container to the writer
pub fn write_hlc_container<W: Write>(
    writer: &mut W,
    chunks: &[CompressedChunk],
    config: &HlcConfig,
) -> Result<u64, HlcError> {
    let mut total_bytes_written = 0u64;

    // Calculate totals
    let total_original_size: u64 = chunks.iter().map(|c| c.original_size as u64).sum();
    let total_compressed_size: u64 = chunks.iter().map(|c| c.compressed_size as u64).sum();

    // Write container header
    let mut header = ContainerHeader::new(config.checksum, chunks.len());
    header.original_size = total_original_size;
    header.compressed_size = total_compressed_size;
    
    header.write(writer)?;
    total_bytes_written += ContainerHeader::size() as u64;

    // Write chunk headers and data
    for chunk in chunks {
        // Chunk header: flags(1) + original_size(4) + compressed_size(4) + checksum(8)
        writer.write_u8(chunk.flags.bits())?;
        writer.write_u32::<LittleEndian>(chunk.original_size)?;
        writer.write_u32::<LittleEndian>(chunk.compressed_size)?;
        writer.write_u64::<LittleEndian>(chunk.original_checksum)?;
        
        // Chunk data
        writer.write_all(&chunk.data)?;
        
        total_bytes_written += (1 + 4 + 4 + 8) + chunk.data.len() as u64;
    }

    Ok(total_bytes_written)
}

/// Reads the complete HLC container from the reader
pub fn read_hlc_container<R: Read>(
    reader: &mut R,
) -> Result<(Vec<CompressedChunk>, HlcConfig), HlcError> {
    // Read container header
    let header = ContainerHeader::read(reader)?;

    let config = HlcConfig {
        checksum: header.checksum_type,
        ..Default::default()
    };

    let mut chunks = Vec::with_capacity(header.chunk_count as usize);

    // Read chunks
    for id in 0..header.chunk_count {
        // Read chunk header
        let flags_byte = reader.read_u8()?;
        let flags = PipelineFlags::from_bits_truncate(flags_byte);
        let original_size = reader.read_u32::<LittleEndian>()?;
        let compressed_size = reader.read_u32::<LittleEndian>()?;
        let original_checksum = reader.read_u64::<LittleEndian>()?;

        // Read chunk data
        let mut data = vec![0; compressed_size as usize];
        reader.read_exact(&mut data)?;

        chunks.push(CompressedChunk {
            id: id as usize,
            flags,
            original_checksum,
            original_size,
            compressed_size,
            data,
        });
    }

    Ok((chunks, config))
}

/// Calculate checksum for data
pub fn calculate_checksum(data: &[u8], checksum_type: ChecksumType) -> u64 {
    match checksum_type {
        ChecksumType::CRC32 => {
            let mut hasher = Crc32Hasher::new();
            hasher.update(data);
            hasher.finalize() as u64
        }
        ChecksumType::SHA256 => {
            // Truncate SHA256 to 64 bits for storage efficiency
            let mut hasher = Sha256::new();
            hasher.update(data);
            let hash = hasher.finalize();
            u64::from_le_bytes(hash[0..8].try_into().unwrap())
        }
    }
}

/// Verify container integrity
pub fn verify_container<R: Read>(reader: &mut R) -> Result<bool, HlcError> {
    let (chunks, config) = read_hlc_container(reader)?;
    
    for chunk in chunks {
        // Try to decompress each chunk to verify integrity
        chunk.decompress(&config)?;
    }
    
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_container_header_roundtrip() {
        let original = ContainerHeader::new(ChecksumType::CRC32, 5);
        
        let mut buffer = Vec::new();
        original.write(&mut buffer).unwrap();
        
        let mut cursor = Cursor::new(buffer);
        let decoded = ContainerHeader::read(&mut cursor).unwrap();
        
        assert_eq!(original.version, decoded.version);
        assert_eq!(original.checksum_type, decoded.checksum_type);
        assert_eq!(original.chunk_count, decoded.chunk_count);
    }

    #[test]
    fn test_pipeline_flags() {
        let flags = PipelineFlags::RLE | PipelineFlags::DELTA | PipelineFlags::ENTROPY;
        
        assert!(flags.contains(PipelineFlags::RLE));
        assert!(flags.contains(PipelineFlags::DELTA));
        assert!(flags.contains(PipelineFlags::ENTROPY));
        assert!(!flags.contains(PipelineFlags::STORED));
        
        let bits = flags.bits();
        let restored = PipelineFlags::from_bits_truncate(bits);
        assert_eq!(flags, restored);
    }

    #[test]
    fn test_checksum_calculation() {
        let data = b"Hello, world!";
        
        let crc32 = calculate_checksum(data, ChecksumType::CRC32);
        let sha256 = calculate_checksum(data, ChecksumType::SHA256);
        
        // Should be different
        assert_ne!(crc32, sha256);
        
        // Should be consistent
        assert_eq!(crc32, calculate_checksum(data, ChecksumType::CRC32));
        assert_eq!(sha256, calculate_checksum(data, ChecksumType::SHA256));
    }

    #[test]
    fn test_compressed_chunk() {
        let data = b"Test data for compression".to_vec();
        let checksum = calculate_checksum(&data, ChecksumType::CRC32);
        let chunk = CompressedChunk::new(0, data.clone(), data.len(), checksum);
        
        assert_eq!(chunk.original_size as usize, data.len());
        assert_eq!(chunk.compressed_size as usize, data.len());
        assert_eq!(chunk.compression_ratio(), 1.0);
        
        let config = HlcConfig::default();
        let decompressed = chunk.decompress(&config).unwrap();
        assert_eq!(decompressed.data, data);
    }
}