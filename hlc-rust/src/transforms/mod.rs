//! Transform modules for data preprocessing before entropy coding
//! 
//! Implements various transforms like delta encoding, RLE, dictionary substitution,
//! and XOR encoding to improve compression ratios.

pub mod delta;
pub mod dictionary;
pub mod rle;

use anyhow::Result;
use crate::analyzer::TransformRecommendation;

/// Trait for all transform implementations
pub trait Transform {
    /// Apply the transform to input data
    fn encode(&self, data: &[u8]) -> Result<Vec<u8>>;
    
    /// Reverse the transform to recover original data
    fn decode(&self, data: &[u8]) -> Result<Vec<u8>>;
    
    /// Get the transform identifier
    fn id(&self) -> u8;
}

/// Transform manager that applies multiple transforms in sequence
pub struct TransformPipeline {
    transforms: Vec<Box<dyn Transform + Send + Sync>>,
}

impl TransformPipeline {
    pub fn new() -> Self {
        Self {
            transforms: Vec::new(),
        }
    }

    /// Add a transform to the pipeline
    pub fn add_transform(&mut self, transform: Box<dyn Transform + Send + Sync>) {
        self.transforms.push(transform);
    }

    /// Build pipeline from recommendations
    pub fn from_recommendations(recommendations: &[TransformRecommendation]) -> Self {
        let mut pipeline = Self::new();
        
        for recommendation in recommendations {
            match recommendation {
                TransformRecommendation::Delta => {
                    pipeline.add_transform(Box::new(delta::DeltaTransform::new()));
                }
                TransformRecommendation::RLE => {
                    pipeline.add_transform(Box::new(rle::RLETransform::new()));
                }
                TransformRecommendation::Dictionary => {
                    pipeline.add_transform(Box::new(dictionary::DictionaryTransform::new()));
                }
                TransformRecommendation::XOR => {
                    // TODO: Implement XOR transform
                }
                TransformRecommendation::None => {
                    // No transform needed
                }
            }
        }
        
        pipeline
    }

    /// Apply all transforms in sequence
    pub fn encode(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut result = data.to_vec();
        
        for transform in &self.transforms {
            result = transform.encode(&result)?;
        }
        
        Ok(result)
    }

    /// Reverse all transforms in reverse order
    pub fn decode(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut result = data.to_vec();
        
        for transform in self.transforms.iter().rev() {
            result = transform.decode(&result)?;
        }
        
        Ok(result)
    }

    /// Get the combined transform flags
    pub fn get_flags(&self) -> u8 {
        let mut flags = 0u8;
        for transform in &self.transforms {
            flags |= 1 << (transform.id() - 1);
        }
        flags
    }

    /// Create pipeline from transform flags
    pub fn from_flags(flags: u8) -> Self {
        let mut pipeline = Self::new();
        
        for i in 0..8 {
            if (flags & (1 << i)) != 0 {
                match i + 1 {
                    1 => pipeline.add_transform(Box::new(delta::DeltaTransform::new())),
                    2 => pipeline.add_transform(Box::new(rle::RLETransform::new())),
                    4 => pipeline.add_transform(Box::new(dictionary::DictionaryTransform::new())),
                    _ => {} // Unknown transform ID
                }
            }
        }
        
        pipeline
    }
}

impl Default for TransformPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_pipeline_roundtrip() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_transform(Box::new(delta::DeltaTransform::new()));
        pipeline.add_transform(Box::new(rle::RLETransform::new()));
        
        let original = b"Hello, world! This is a test.";
        let encoded = pipeline.encode(original).unwrap();
        let decoded = pipeline.decode(&encoded).unwrap();
        
        assert_eq!(original, &decoded[..]);
    }

    #[test]
    fn test_flags_roundtrip() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_transform(Box::new(delta::DeltaTransform::new()));
        pipeline.add_transform(Box::new(rle::RLETransform::new()));
        
        let flags = pipeline.get_flags();
        let new_pipeline = TransformPipeline::from_flags(flags);
        
        let original = b"Test data for flags roundtrip";
        let encoded1 = pipeline.encode(original).unwrap();
        let encoded2 = new_pipeline.encode(original).unwrap();
        
        assert_eq!(encoded1, encoded2);
    }
}