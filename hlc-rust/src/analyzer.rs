//! File type analysis and content detection
//! 
//! Analyzes input data to determine optimal compression strategies
//! and transform methods.


/// Content type detected by the analyzer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Unknown,
    Text,
    Json,
    Xml,
    Binary,
    Image,
    Audio,
    Video,
    Archive,
}

/// Analysis result for a chunk of data
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub content_type: ContentType,
    pub entropy: f64,
    pub compression_hint: CompressionHint,
    pub recommended_transforms: Vec<TransformRecommendation>,
}

/// Compression strategy hints
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionHint {
    Fast,      // Use fast compression for real-time applications
    Balanced,  // Balance speed and ratio
    Max,       // Maximum compression ratio
}

/// Recommended transform methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformRecommendation {
    None,
    Delta,
    RLE,
    Dictionary,
    XOR,
}

/// Content analyzer
pub struct ContentAnalyzer {
    sample_size: usize,
}

impl ContentAnalyzer {
    pub fn new() -> Self {
        Self {
            sample_size: 8192, // Analyze first 8KB
        }
    }

    /// Analyze a chunk of data to determine optimal compression strategy
    pub fn analyze_chunk(&self, data: &[u8]) -> AnalysisResult {
        let content_type = self.detect_content_type(data);
        let entropy = self.calculate_entropy(data);
        let compression_hint = self.determine_compression_hint(&content_type, entropy);
        let recommended_transforms = self.recommend_transforms(&content_type, entropy);

        AnalysisResult {
            content_type,
            entropy,
            compression_hint,
            recommended_transforms,
        }
    }

    /// Detect content type based on data patterns
    fn detect_content_type(&self, data: &[u8]) -> ContentType {
        if data.is_empty() {
            return ContentType::Unknown;
        }

        // Check for text content
        if self.is_text(data) {
            if self.is_json(data) {
                return ContentType::Json;
            }
            if self.is_xml(data) {
                return ContentType::Xml;
            }
            return ContentType::Text;
        }

        // Check for binary content types
        if self.is_image(data) {
            return ContentType::Image;
        }
        if self.is_audio(data) {
            return ContentType::Audio;
        }
        if self.is_video(data) {
            return ContentType::Video;
        }
        if self.is_archive(data) {
            return ContentType::Archive;
        }

        ContentType::Binary
    }

    /// Check if data appears to be text
    fn is_text(&self, data: &[u8]) -> bool {
        if data.is_empty() {
            return false;
        }

        let sample_size = data.len().min(self.sample_size);
        let sample = &data[..sample_size];
        
        // Count printable ASCII characters
        let printable_count = sample.iter()
            .filter(|&&b| b >= 32 && b <= 126 || b == 9 || b == 10 || b == 13)
            .count();
        
        // Consider it text if at least 80% of characters are printable
        (printable_count as f64 / sample_size as f64) >= 0.8
    }

    /// Check if data appears to be JSON
    fn is_json(&self, data: &[u8]) -> bool {
        let sample_size = data.len().min(self.sample_size);
        let sample = &data[..sample_size];
        
        // Look for JSON patterns
        let sample_str = match std::str::from_utf8(sample) {
            Ok(s) => s,
            Err(_) => return false,
        };

        sample_str.trim_start().starts_with('{') || 
        sample_str.trim_start().starts_with('[')
    }

    /// Check if data appears to be XML
    fn is_xml(&self, data: &[u8]) -> bool {
        let sample_size = data.len().min(self.sample_size);
        let sample = &data[..sample_size];
        
        let sample_str = match std::str::from_utf8(sample) {
            Ok(s) => s,
            Err(_) => return false,
        };

        sample_str.trim_start().starts_with('<') &&
        (sample_str.contains("<?xml") || sample_str.contains('<') && sample_str.contains('>'))
    }

    /// Check if data appears to be an image
    fn is_image(&self, data: &[u8]) -> bool {
        if data.len() < 4 {
            return false;
        }

        // Check for common image file signatures
        let header = &data[..4];
        header == b"\xFF\xD8\xFF" || // JPEG
        header == b"\x89PNG" ||      // PNG
        header == b"GIF8" ||         // GIF
        header == b"RIFF" ||         // WebP (RIFF format)
        (header[0] == 0x42 && header[1] == 0x4D) // BMP
    }

    /// Check if data appears to be audio
    fn is_audio(&self, data: &[u8]) -> bool {
        if data.len() < 12 {
            return false;
        }

        let header = &data[..12];
        header.starts_with(b"RIFF") && header[8..12] == *b"WAVE" || // WAV
        header.starts_with(b"OggS") || // OGG
        header.starts_with(b"ID3") ||  // MP3
        header.starts_with(b"\xFF\xFB") // MP3
    }

    /// Check if data appears to be video
    fn is_video(&self, data: &[u8]) -> bool {
        if data.len() < 12 {
            return false;
        }

        let header = &data[..12];
        header.starts_with(b"RIFF") && header[8..12] == *b"AVI " || // AVI
        header.starts_with(b"\x00\x00\x00\x20ftypmp42") || // MP4
        header.starts_with(b"\x00\x00\x00\x18ftypisom") // MP4
    }

    /// Check if data appears to be an archive
    fn is_archive(&self, data: &[u8]) -> bool {
        if data.len() < 4 {
            return false;
        }

        let header = &data[..4];
        header == b"PK\x03\x04" || // ZIP
        header == b"PK\x05\x06" || // ZIP (empty)
        header == b"PK\x07\x08" || // ZIP (spanned)
        header == b"\x1F\x8B\x08" || // GZIP
        header == b"Rar!" || // RAR
        header == b"\x37\x7A\xBC\xAF" // 7Z
    }

    /// Calculate Shannon entropy
    fn calculate_entropy(&self, data: &[u8]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }

        let mut counts = [0u32; 256];
        for &byte in data {
            counts[byte as usize] += 1;
        }

        let mut entropy = 0.0;
        let data_len = data.len() as f64;
        
        for &count in counts.iter() {
            if count > 0 {
                let probability = count as f64 / data_len;
                entropy -= probability * probability.log2();
            }
        }

        entropy
    }

    /// Determine compression hint based on content type and entropy
    fn determine_compression_hint(&self, content_type: &ContentType, entropy: f64) -> CompressionHint {
        match content_type {
            ContentType::Json | ContentType::Xml | ContentType::Text => {
                if entropy > 7.0 {
                    CompressionHint::Max // High entropy text benefits from max compression
                } else {
                    CompressionHint::Balanced
                }
            }
            ContentType::Image | ContentType::Audio | ContentType::Video => {
                CompressionHint::Fast // Media files are often already compressed
            }
            ContentType::Binary => {
                if entropy > 7.5 {
                    CompressionHint::Max // High entropy binary
                } else {
                    CompressionHint::Balanced
                }
            }
            _ => CompressionHint::Balanced,
        }
    }

    /// Recommend transform methods based on content analysis
    fn recommend_transforms(&self, content_type: &ContentType, entropy: f64) -> Vec<TransformRecommendation> {
        let mut recommendations = Vec::new();

        match content_type {
            ContentType::Json | ContentType::Xml => {
                recommendations.push(TransformRecommendation::Dictionary);
                if entropy < 6.0 {
                    recommendations.push(TransformRecommendation::RLE);
                }
            }
            ContentType::Text => {
                if entropy < 6.0 {
                    recommendations.push(TransformRecommendation::RLE);
                }
                recommendations.push(TransformRecommendation::Dictionary);
            }
            ContentType::Binary => {
                if entropy < 5.0 {
                    recommendations.push(TransformRecommendation::RLE);
                }
                recommendations.push(TransformRecommendation::Delta);
            }
            _ => {
                // For unknown or already compressed content, use minimal transforms
                if entropy < 4.0 {
                    recommendations.push(TransformRecommendation::RLE);
                }
            }
        }

        recommendations
    }
}

impl Default for ContentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_detection() {
        let analyzer = ContentAnalyzer::new();
        let json_data = b"{\"name\": \"test\", \"value\": 42}";
        let result = analyzer.analyze_chunk(json_data);
        assert_eq!(result.content_type, ContentType::Json);
        assert!(result.recommended_transforms.contains(&TransformRecommendation::Dictionary));
    }

    #[test]
    fn test_text_detection() {
        let analyzer = ContentAnalyzer::new();
        let text_data = b"Hello, world! This is plain text.";
        let result = analyzer.analyze_chunk(text_data);
        assert_eq!(result.content_type, ContentType::Text);
    }

    #[test]
    fn test_binary_detection() {
        let analyzer = ContentAnalyzer::new();
        let binary_data = &[0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let result = analyzer.analyze_chunk(binary_data);
        assert_eq!(result.content_type, ContentType::Binary);
    }

    #[test]
    fn test_entropy_calculation() {
        let analyzer = ContentAnalyzer::new();
        
        // Low entropy
        let low_entropy = vec![0u8; 1000];
        let result = analyzer.analyze_chunk(&low_entropy);
        assert!(result.entropy < 0.1);
        
        // High entropy
        let high_entropy: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let result = analyzer.analyze_chunk(&high_entropy);
        assert!(result.entropy > 7.0);
    }
}