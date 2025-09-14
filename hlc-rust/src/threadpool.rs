//! Thread pool implementation for parallel compression
//! 
//! Provides a custom thread pool for parallel chunk processing while
//! maintaining deterministic output ordering.

use anyhow::Result;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use crate::chunker::Chunk;
use crate::analyzer::{ContentAnalyzer, AnalysisResult};
use crate::transforms::TransformPipeline;
use crate::entropy::EntropyCoderFactory;
use crate::container::EntropyMethod;
use crate::checksum::{ChecksumCalculator, ChecksumType};
use crate::container::ChunkHeader;

/// Task for processing a chunk
#[derive(Debug)]
pub struct ChunkTask {
    pub chunk: Chunk,
    pub analysis: AnalysisResult,
}

/// Result of chunk processing
#[derive(Debug)]
pub struct ChunkResult {
    pub chunk_id: u32,
    pub original_data: Vec<u8>,
    pub compressed_data: Vec<u8>,
    pub header: ChunkHeader,
}

/// Thread pool for parallel chunk processing
pub struct CompressionThreadPool {
    task_sender: Sender<ChunkTask>,
    result_receiver: Receiver<ChunkResult>,
    workers: Vec<thread::JoinHandle<()>>,
}

impl CompressionThreadPool {
    pub fn new(num_threads: usize) -> Self {
        let (task_sender, task_receiver) = bounded::<ChunkTask>(1000);
        let (result_sender, result_receiver) = bounded::<ChunkResult>(1000);
        
        let task_receiver = Arc::new(task_receiver);
        let result_sender = Arc::new(result_sender);
        
        let mut workers = Vec::new();
        
        for worker_id in 0..num_threads {
            let task_receiver = Arc::clone(&task_receiver);
            let result_sender = Arc::clone(&result_sender);
            
            let worker = thread::spawn(move || {
                Self::worker_loop(worker_id, task_receiver, result_sender);
            });
            
            workers.push(worker);
        }
        
        Self {
            task_sender,
            result_receiver,
            workers,
        }
    }

    /// Submit a chunk for processing
    pub fn submit_chunk(&self, chunk: Chunk, analysis: AnalysisResult) -> Result<()> {
        self.task_sender.send(ChunkTask { chunk, analysis })
            .map_err(|e| anyhow::anyhow!("Failed to submit chunk: {}", e))?;
        Ok(())
    }

    /// Get the next completed chunk result
    pub fn get_result(&self) -> Option<ChunkResult> {
        self.result_receiver.try_recv().ok()
    }

    /// Get the next completed chunk result (blocking)
    pub fn get_result_blocking(&self) -> Result<ChunkResult> {
        self.result_receiver.recv()
            .map_err(|e| anyhow::anyhow!("Failed to receive result: {}", e))
    }

    /// Check if there are pending results
    pub fn has_pending_results(&self) -> bool {
        !self.result_receiver.is_empty()
    }

    /// Shutdown the thread pool
    pub fn shutdown(self) -> Result<()> {
        drop(self.task_sender); // Close the task channel
        
        for worker in self.workers {
            worker.join()
                .map_err(|e| anyhow::anyhow!("Worker thread panicked: {:?}", e))?;
        }
        
        Ok(())
    }

    fn worker_loop(
        worker_id: usize,
        task_receiver: Arc<Receiver<ChunkTask>>,
        result_sender: Arc<Sender<ChunkResult>>,
    ) {
        log::debug!("Worker {} started", worker_id);
        
        while let Ok(task) = task_receiver.recv() {
            match Self::process_chunk(task) {
                Ok(result) => {
                    if let Err(e) = result_sender.send(result) {
                        log::error!("Worker {} failed to send result: {}", worker_id, e);
                        break;
                    }
                }
                Err(e) => {
                    log::error!("Worker {} failed to process chunk: {}", worker_id, e);
                }
            }
        }
        
        log::debug!("Worker {} finished", worker_id);
    }

    fn process_chunk(task: ChunkTask) -> Result<ChunkResult> {
        let ChunkTask { chunk, analysis } = task;
        
        // Create transform pipeline based on analysis
        let mut transform_pipeline = TransformPipeline::from_recommendations(&analysis.recommended_transforms);
        
        // Apply transforms
        let transformed_data = transform_pipeline.encode(&chunk.data)?;
        
        // Choose entropy coder based on analysis
        let entropy_method = match analysis.compression_hint {
            crate::analyzer::CompressionHint::Fast => EntropyMethod::LZ4,
            crate::analyzer::CompressionHint::Balanced => EntropyMethod::Zstd,
            crate::analyzer::CompressionHint::Max => EntropyMethod::RangeCoder,
        };
        
        let entropy_coder = EntropyCoderFactory::create(entropy_method, None);
        let compressed_data = entropy_coder.encode(&transformed_data)?;
        
        // Calculate checksums
        let crc32_calculator = ChecksumCalculator::new(ChecksumType::Crc32);
        let crc32_result = crc32_calculator.calculate(&chunk.data)?;
        
        let crc32 = match crc32_result {
            crate::checksum::ChecksumResult::Crc32(crc) => crc,
            _ => return Err(anyhow::anyhow!("Expected CRC32 result")),
        };
        
        // Create chunk header
        let transform_flags = transform_pipeline.get_flags();
        let entropy_method_byte = entropy_method as u8;
        
        let header = ChunkHeader::new(
            chunk.id,
            chunk.data.len() as u32,
            compressed_data.len() as u32,
            transform_flags,
            entropy_method_byte,
            crc32,
            None, // SHA256 not implemented yet
        );
        
        Ok(ChunkResult {
            chunk_id: chunk.id,
            original_data: chunk.data,
            compressed_data,
            header,
        })
    }
}

/// Parallel chunk processor using Rayon
pub struct RayonChunkProcessor {
    analyzer: ContentAnalyzer,
}

impl RayonChunkProcessor {
    pub fn new() -> Self {
        Self {
            analyzer: ContentAnalyzer::new(),
        }
    }

    /// Process chunks in parallel using Rayon
    pub fn process_chunks_parallel(&self, chunks: Vec<Chunk>) -> Result<Vec<ChunkResult>> {
        use rayon::prelude::*;
        
        let results: Result<Vec<_>> = chunks
            .into_par_iter()
            .map(|chunk| {
                let analysis = self.analyzer.analyze_chunk(&chunk.data);
                Self::process_single_chunk(chunk, analysis)
            })
            .collect();
        
        results
    }

    fn process_single_chunk(chunk: Chunk, analysis: AnalysisResult) -> Result<ChunkResult> {
        // Create transform pipeline based on analysis
        let mut transform_pipeline = TransformPipeline::from_recommendations(&analysis.recommended_transforms);
        
        // Apply transforms
        let transformed_data = transform_pipeline.encode(&chunk.data)?;
        
        // Choose entropy coder based on analysis
        let entropy_method = match analysis.compression_hint {
            crate::analyzer::CompressionHint::Fast => EntropyMethod::LZ4,
            crate::analyzer::CompressionHint::Balanced => EntropyMethod::Zstd,
            crate::analyzer::CompressionHint::Max => EntropyMethod::RangeCoder,
        };
        
        let entropy_coder = EntropyCoderFactory::create(entropy_method, None);
        let compressed_data = entropy_coder.encode(&transformed_data)?;
        
        // Calculate checksums
        let crc32_calculator = ChecksumCalculator::new(ChecksumType::Crc32);
        let crc32_result = crc32_calculator.calculate(&chunk.data)?;
        
        let crc32 = match crc32_result {
            crate::checksum::ChecksumResult::Crc32(crc) => crc,
            _ => return Err(anyhow::anyhow!("Expected CRC32 result")),
        };
        
        // Create chunk header
        let transform_flags = transform_pipeline.get_flags();
        let entropy_method_byte = entropy_method as u8;
        
        let header = ChunkHeader::new(
            chunk.id,
            chunk.data.len() as u32,
            compressed_data.len() as u32,
            transform_flags,
            entropy_method_byte,
            crc32,
            None, // SHA256 not implemented yet
        );
        
        Ok(ChunkResult {
            chunk_id: chunk.id,
            original_data: chunk.data,
            compressed_data,
            header,
        })
    }
}

impl Default for RayonChunkProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread pool statistics
#[derive(Debug, Clone)]
pub struct ThreadPoolStats {
    pub num_workers: usize,
    pub pending_tasks: usize,
    pub completed_tasks: usize,
}

impl CompressionThreadPool {
    /// Get thread pool statistics
    pub fn stats(&self) -> ThreadPoolStats {
        ThreadPoolStats {
            num_workers: self.workers.len(),
            pending_tasks: self.task_sender.len(),
            completed_tasks: self.result_receiver.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunker::Chunk;

    #[test]
    fn test_thread_pool_processing() {
        let pool = CompressionThreadPool::new(2);
        
        // Create test chunks
        let chunks = vec![
            Chunk::new(0, b"Hello, world!".to_vec()),
            Chunk::new(1, b"This is a test.".to_vec()),
        ];
        
        let analyzer = ContentAnalyzer::new();
        
        // Submit chunks
        for chunk in chunks {
            let analysis = analyzer.analyze_chunk(&chunk.data);
            pool.submit_chunk(chunk, analysis).unwrap();
        }
        
        // Collect results
        let mut results = Vec::new();
        while let Some(result) = pool.get_result() {
            results.push(result);
        }
        
        // Shutdown pool
        pool.shutdown().unwrap();
        
        assert_eq!(results.len(), 2);
        results.sort_by_key(|r| r.chunk_id);
        assert_eq!(results[0].chunk_id, 0);
        assert_eq!(results[1].chunk_id, 1);
    }

    #[test]
    fn test_rayon_processor() {
        let processor = RayonChunkProcessor::new();
        
        let chunks = vec![
            Chunk::new(0, b"Hello, world!".to_vec()),
            Chunk::new(1, b"This is a test.".to_vec()),
        ];
        
        let results = processor.process_chunks_parallel(chunks).unwrap();
        assert_eq!(results.len(), 2);
    }
}