//! Container format for HLC files
//! 
//! The container format uses a deterministic layout with per-chunk headers
//! immediately before their payloads, plus an optional chunk index at the end.

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// Magic number for HLC files
pub const HLC_MAGIC: [u8; 8] = [b'H', b'L', b'C', 0x00, 0x01, 0x00, 0x00, 0x00];

/// Current format version
pub const FORMAT_VERSION: u16 = 1;

/// Global file header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalHeader {
    pub magic: [u8; 8],
    pub version: u16,
    pub flags: u32,
    pub total_chunks: u32,
    pub total_size: u64,
    pub compressed_size: u64,
    pub created_timestamp: u64,
}

impl GlobalHeader {
    pub fn new(total_chunks: u32, total_size: u64, compressed_size: u64) -> Self {
        Self {
            magic: HLC_MAGIC,
            version: FORMAT_VERSION,
            flags: 0,
            total_chunks,
            total_size,
            compressed_size,
            created_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.magic)?;
        writer.write_u16::<LittleEndian>(self.version)?;
        writer.write_u32::<LittleEndian>(self.flags)?;
        writer.write_u32::<LittleEndian>(self.total_chunks)?;
        writer.write_u64::<LittleEndian>(self.total_size)?;
        writer.write_u64::<LittleEndian>(self.compressed_size)?;
        writer.write_u64::<LittleEndian>(self.created_timestamp)?;
        Ok(())
    }

    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        
        if magic != HLC_MAGIC {
            anyhow::bail!("Invalid HLC magic number");
        }

        let version = reader.read_u16::<LittleEndian>()?;
        if version != FORMAT_VERSION {
            anyhow::bail!("Unsupported format version: {}", version);
        }

        let flags = reader.read_u32::<LittleEndian>()?;
        let total_chunks = reader.read_u32::<LittleEndian>()?;
        let total_size = reader.read_u64::<LittleEndian>()?;
        let compressed_size = reader.read_u64::<LittleEndian>()?;
        let created_timestamp = reader.read_u64::<LittleEndian>()?;

        Ok(Self {
            magic,
            version,
            flags,
            total_chunks,
            total_size,
            compressed_size,
            created_timestamp,
        })
    }
}

/// Per-chunk header immediately before payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkHeader {
    pub chunk_id: u32,
    pub original_size: u32,
    pub compressed_size: u32,
    pub transform_flags: u8,
    pub entropy_method: u8,
    pub crc32: u32,
    pub sha256: Option<[u8; 32]>,
    pub reserved: [u8; 2], // For future extensions
}

impl ChunkHeader {
    pub fn new(
        chunk_id: u32,
        original_size: u32,
        compressed_size: u32,
        transform_flags: u8,
        entropy_method: u8,
        crc32: u32,
        sha256: Option<[u8; 32]>,
    ) -> Self {
        Self {
            chunk_id,
            original_size,
            compressed_size,
            transform_flags,
            entropy_method,
            crc32,
            sha256,
            reserved: [0; 2],
        }
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32::<LittleEndian>(self.chunk_id)?;
        writer.write_u32::<LittleEndian>(self.original_size)?;
        writer.write_u32::<LittleEndian>(self.compressed_size)?;
        writer.write_u8(self.transform_flags)?;
        writer.write_u8(self.entropy_method)?;
        writer.write_u32::<LittleEndian>(self.crc32)?;
        
        if let Some(sha256) = self.sha256 {
            writer.write_all(&sha256)?;
        } else {
            writer.write_all(&[0u8; 32])?;
        }
        
        writer.write_all(&self.reserved)?;
        Ok(())
    }

    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let chunk_id = reader.read_u32::<LittleEndian>()?;
        let original_size = reader.read_u32::<LittleEndian>()?;
        let compressed_size = reader.read_u32::<LittleEndian>()?;
        let transform_flags = reader.read_u8()?;
        let entropy_method = reader.read_u8()?;
        let crc32 = reader.read_u32::<LittleEndian>()?;
        
        let mut sha256_bytes = [0u8; 32];
        reader.read_exact(&mut sha256_bytes)?;
        let sha256 = if sha256_bytes.iter().any(|&b| b != 0) {
            Some(sha256_bytes)
        } else {
            None
        };
        
        let mut reserved = [0u8; 2];
        reader.read_exact(&mut reserved)?;

        Ok(Self {
            chunk_id,
            original_size,
            compressed_size,
            transform_flags,
            entropy_method,
            crc32,
            sha256,
            reserved,
        })
    }

    pub fn size() -> usize {
        4 + 4 + 4 + 1 + 1 + 4 + 32 + 2 // All fields combined
    }
}

/// Chunk index entry for random access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkIndexEntry {
    pub chunk_id: u32,
    pub offset: u64,
    pub header_size: u32,
    pub payload_size: u32,
}

impl ChunkIndexEntry {
    pub fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32::<LittleEndian>(self.chunk_id)?;
        writer.write_u64::<LittleEndian>(self.offset)?;
        writer.write_u32::<LittleEndian>(self.header_size)?;
        writer.write_u32::<LittleEndian>(self.payload_size)?;
        Ok(())
    }

    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let chunk_id = reader.read_u32::<LittleEndian>()?;
        let offset = reader.read_u64::<LittleEndian>()?;
        let header_size = reader.read_u32::<LittleEndian>()?;
        let payload_size = reader.read_u32::<LittleEndian>()?;
        Ok(Self {
            chunk_id,
            offset,
            header_size,
            payload_size,
        })
    }
}

/// Transform method flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformMethod {
    None = 0,
    Delta = 1,
    RLE = 2,
    Dictionary = 4,
    XOR = 8,
}

/// Entropy coding methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntropyMethod {
    None = 0,
    RangeCoder = 1,
    ANS = 2,
    LZ4 = 3,
    Zstd = 4,
}

impl From<u8> for TransformMethod {
    fn from(flags: u8) -> Self {
        match flags {
            0 => TransformMethod::None,
            1 => TransformMethod::Delta,
            2 => TransformMethod::RLE,
            4 => TransformMethod::Dictionary,
            8 => TransformMethod::XOR,
            _ => TransformMethod::None,
        }
    }
}

impl From<u8> for EntropyMethod {
    fn from(method: u8) -> Self {
        match method {
            0 => EntropyMethod::None,
            1 => EntropyMethod::RangeCoder,
            2 => EntropyMethod::ANS,
            3 => EntropyMethod::LZ4,
            4 => EntropyMethod::Zstd,
            _ => EntropyMethod::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_global_header_roundtrip() {
        let header = GlobalHeader::new(100, 1024 * 1024, 512 * 1024);
        let mut buffer = Vec::new();
        header.write(&mut buffer).unwrap();
        
        let mut cursor = Cursor::new(&buffer);
        let read_header = GlobalHeader::read(&mut cursor).unwrap();
        
        assert_eq!(header.magic, read_header.magic);
        assert_eq!(header.version, read_header.version);
        assert_eq!(header.total_chunks, read_header.total_chunks);
    }

    #[test]
    fn test_chunk_header_roundtrip() {
        let header = ChunkHeader::new(
            42,
            1024,
            512,
            TransformMethod::Delta as u8,
            EntropyMethod::RangeCoder as u8,
            0x12345678,
            None,
        );
        let mut buffer = Vec::new();
        header.write(&mut buffer).unwrap();
        
        let mut cursor = Cursor::new(&buffer);
        let read_header = ChunkHeader::read(&mut cursor).unwrap();
        
        assert_eq!(header.chunk_id, read_header.chunk_id);
        assert_eq!(header.original_size, read_header.original_size);
        assert_eq!(header.compressed_size, read_header.compressed_size);
        assert_eq!(header.crc32, read_header.crc32);
    }
}