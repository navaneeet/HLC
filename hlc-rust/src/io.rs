//! I/O utilities for ordered writing and streaming
//! 
//! Provides deterministic writing of compressed chunks to avoid file corruption
//! and ensure proper ordering during parallel compression.

use anyhow::Result;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::io::{Write, Read, Seek, SeekFrom};
use std::path::Path;
use std::fs::File;
use tokio::task;
use crate::container::{ChunkHeader, GlobalHeader};

/// Chunk data ready for writing
#[derive(Debug, Clone)]
pub struct ChunkData {
    pub chunk_id: u32,
    pub original_data: Vec<u8>,
    pub compressed_data: Vec<u8>,
    pub header: ChunkHeader,
}

/// Ordered writer that ensures chunks are written in the correct sequence
pub struct OrderedWriter {
    sender: Sender<ChunkData>,
    writer_task: Option<task::JoinHandle<Result<()>>>,
}

impl OrderedWriter {
    pub fn new<W: Write + Send + 'static>(writer: W) -> Self {
        let (sender, receiver) = bounded::<ChunkData>(1000); // Buffer up to 1000 chunks
        
        let writer_task = task::spawn_blocking(move || {
            Self::writer_loop(writer, receiver)
        });

        Self {
            sender,
            writer_task: Some(writer_task),
        }
    }

    /// Write a chunk (will be written in order)
    pub async fn write_chunk(&self, chunk_data: ChunkData) -> Result<()> {
        self.sender.send(chunk_data)
            .map_err(|e| anyhow::anyhow!("Failed to send chunk: {}", e))?;
        Ok(())
    }

    /// Finish writing and wait for completion
    pub async fn finish(self) -> Result<()> {
        drop(self.sender); // Close the channel
        
        if let Some(task) = self.writer_task {
            task.await??;
        }
        
        Ok(())
    }

    fn writer_loop<W: Write + Send>(mut writer: W, receiver: Receiver<ChunkData>) -> Result<()> {
        let mut expected_chunk_id = 0;
        let mut pending_chunks = std::collections::BTreeMap::new();

        while let Ok(chunk_data) = receiver.recv() {
            pending_chunks.insert(chunk_data.chunk_id, chunk_data);

            // Write chunks in order
            while let Some(&chunk_id) = pending_chunks.keys().next() {
                if chunk_id == expected_chunk_id {
                    if let Some(chunk_data) = pending_chunks.remove(&chunk_id) {
                        Self::write_single_chunk(&mut writer, &chunk_data)?;
                        expected_chunk_id += 1;
                    }
                } else {
                    break; // Wait for the next chunk in sequence
                }
            }
        }

        // Write any remaining chunks
        for (_, chunk_data) in pending_chunks {
            Self::write_single_chunk(&mut writer, &chunk_data)?;
        }

        Ok(())
    }

    fn write_single_chunk<W: Write>(writer: &mut W, chunk_data: &ChunkData) -> Result<()> {
        // Write chunk header
        chunk_data.header.write(writer)?;
        
        // Write compressed data
        writer.write_all(&chunk_data.compressed_data)?;
        
        Ok(())
    }
}

/// Streaming reader for reading compressed files
pub struct StreamingReader {
    file: File,
    global_header: GlobalHeader,
    current_chunk: u32,
}

impl StreamingReader {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path)?;
        
        // Read global header
        let global_header = GlobalHeader::read(&mut file)?;
        
        Ok(Self {
            file,
            global_header,
            current_chunk: 0,
        })
    }

    /// Read the next chunk
    pub fn read_next_chunk(&mut self) -> Result<Option<ChunkData>> {
        if self.current_chunk >= self.global_header.total_chunks {
            return Ok(None);
        }

        // Read chunk header
        let header = ChunkHeader::read(&mut self.file)?;
        
        // Read compressed data
        let mut compressed_data = vec![0u8; header.compressed_size as usize];
        self.file.read_exact(&mut compressed_data)?;

        // For now, we don't have the original data in the reader
        // In a full implementation, this would be handled differently
        let chunk_data = ChunkData {
            chunk_id: header.chunk_id,
            original_data: Vec::new(), // Would need to be populated during decompression
            compressed_data,
            header,
        };

        self.current_chunk += 1;
        Ok(Some(chunk_data))
    }

    /// Get the global header
    pub fn global_header(&self) -> &GlobalHeader {
        &self.global_header
    }

    /// Seek to a specific chunk
    pub fn seek_to_chunk(&mut self, chunk_id: u32) -> Result<()> {
        // This is a simplified implementation
        // In practice, you'd need a chunk index to do this efficiently
        self.file.seek(SeekFrom::Start(GlobalHeader::size() as u64))?;
        self.current_chunk = 0;
        
        // Skip to the desired chunk
        for _ in 0..chunk_id {
            let header = ChunkHeader::read(&mut self.file)?;
            self.file.seek(SeekFrom::Current(header.compressed_size as i64))?;
            self.current_chunk += 1;
        }
        
        Ok(())
    }
}

/// Memory-mapped file reader for efficient random access
pub struct MemoryMappedReader {
    data: memmap2::Mmap,
    global_header: GlobalHeader,
    chunk_offsets: Vec<u64>,
}

impl MemoryMappedReader {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let data = unsafe { memmap2::Mmap::map(&file)? };
        
        // Read global header
        let mut cursor = std::io::Cursor::new(&data);
        let global_header = GlobalHeader::read(&mut cursor)?;
        
        // Build chunk offset table
        let mut chunk_offsets = Vec::new();
        let mut offset = GlobalHeader::size() as u64;
        
        for _ in 0..global_header.total_chunks {
            chunk_offsets.push(offset);
            
            // Read chunk header to get compressed size
            let mut header_data = [0u8; 48]; // ChunkHeader size is 48 bytes
            header_data.copy_from_slice(&data[offset as usize..offset as usize + ChunkHeader::size()]);
            let mut header_cursor = std::io::Cursor::new(&header_data);
            let header = ChunkHeader::read(&mut header_cursor)?;
            
            offset += ChunkHeader::size() as u64 + header.compressed_size as u64;
        }
        
        Ok(Self {
            data,
            global_header,
            chunk_offsets,
        })
    }

    /// Read a specific chunk by ID
    pub fn read_chunk(&self, chunk_id: u32) -> Result<Option<ChunkData>> {
        if chunk_id >= self.global_header.total_chunks {
            return Ok(None);
        }

        let offset = self.chunk_offsets[chunk_id as usize];
        
        // Read chunk header
        let header_start = offset as usize;
        let header_end = header_start + ChunkHeader::size();
        let mut header_cursor = std::io::Cursor::new(&self.data[header_start..header_end]);
        let header = ChunkHeader::read(&mut header_cursor)?;
        
        // Read compressed data
        let data_start = header_end;
        let data_end = data_start + header.compressed_size as usize;
        let compressed_data = self.data[data_start..data_end].to_vec();
        
        Ok(Some(ChunkData {
            chunk_id: header.chunk_id,
            original_data: Vec::new(),
            compressed_data,
            header,
        }))
    }

    /// Get the global header
    pub fn global_header(&self) -> &GlobalHeader {
        &self.global_header
    }
}

/// Utility functions for I/O operations
pub struct IOUtils;

impl IOUtils {
    /// Calculate the size of a global header
    pub fn global_header_size() -> usize {
        GlobalHeader::size()
    }

    /// Calculate the size of a chunk header
    pub fn chunk_header_size() -> usize {
        ChunkHeader::size()
    }

    /// Estimate total file size based on chunks
    pub fn estimate_file_size(_global_header: &GlobalHeader, chunks: &[ChunkData]) -> u64 {
        let header_size = GlobalHeader::size() as u64;
        let chunk_headers_size = (ChunkHeader::size() as u64) * chunks.len() as u64;
        let compressed_data_size: u64 = chunks.iter()
            .map(|chunk| chunk.compressed_data.len() as u64)
            .sum();
        
        header_size + chunk_headers_size + compressed_data_size
    }
}

// Add the missing size() method to GlobalHeader
impl GlobalHeader {
    pub fn size() -> usize {
        8 + 2 + 4 + 4 + 8 + 8 + 8 // All fields combined
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::NamedTempFile;
    use crate::container::{TransformMethod, EntropyMethod};

    #[test]
    fn test_ordered_writer() {
        let buffer = Vec::new();
        let writer = OrderedWriter::new(Cursor::new(buffer));
        
        // Create test chunks (out of order)
        let chunk1 = ChunkData {
            chunk_id: 1,
            original_data: b"chunk1".to_vec(),
            compressed_data: b"compressed1".to_vec(),
            header: ChunkHeader::new(1, 6, 11, TransformMethod::None as u8, EntropyMethod::None as u8, 0x12345678, None),
        };
        
        let chunk0 = ChunkData {
            chunk_id: 0,
            original_data: b"chunk0".to_vec(),
            compressed_data: b"compressed0".to_vec(),
            header: ChunkHeader::new(0, 6, 11, TransformMethod::None as u8, EntropyMethod::None as u8, 0x87654321, None),
        };

        // Write chunks out of order
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            writer.write_chunk(chunk1).await.unwrap();
            writer.write_chunk(chunk0).await.unwrap();
            writer.finish().await.unwrap();
        });

        // Note: In a real test, we would need to access the buffer after writing
        // This test demonstrates the API but doesn't verify the output
    }

    #[test]
    fn test_streaming_reader() {
        // Create a test file
        let temp_file = NamedTempFile::new().unwrap();
        let mut file = temp_file.reopen().unwrap();
        
        // Write test data
        let global_header = GlobalHeader::new(2, 100, 50);
        global_header.write(&mut file).unwrap();
        
        let header0 = ChunkHeader::new(0, 10, 5, TransformMethod::None as u8, EntropyMethod::None as u8, 0x12345678, None);
        header0.write(&mut file).unwrap();
        file.write_all(b"data0").unwrap();
        
        let header1 = ChunkHeader::new(1, 10, 5, TransformMethod::None as u8, EntropyMethod::None as u8, 0x87654321, None);
        header1.write(&mut file).unwrap();
        file.write_all(b"data1").unwrap();
        
        drop(file);
        
        // Read back
        let mut reader = StreamingReader::open(temp_file.path()).unwrap();
        
        let chunk0 = reader.read_next_chunk().unwrap().unwrap();
        assert_eq!(chunk0.chunk_id, 0);
        assert_eq!(chunk0.compressed_data, b"data0");
        
        let chunk1 = reader.read_next_chunk().unwrap().unwrap();
        assert_eq!(chunk1.chunk_id, 1);
        assert_eq!(chunk1.compressed_data, b"data1");
        
        assert!(reader.read_next_chunk().unwrap().is_none());
    }
}