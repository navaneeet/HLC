use crate::config::HlcMode;

#[derive(Debug, Clone)]
pub struct CompressionStrategy {
    pub use_rle: bool,
    pub use_delta: bool,
    pub use_dictionary: bool,
    pub original_data: Vec<u8>,
}

/// Analyzes data chunk to determine the best compression strategy
/// This is a simplified implementation - production versions would use
/// more sophisticated entropy analysis, pattern detection, etc.
pub fn analyze_chunk(data: &[u8], mode: HlcMode) -> CompressionStrategy {
    let mut use_rle = false;
    let mut use_delta = false;
    let mut use_dictionary = false;

    if data.is_empty() {
        return CompressionStrategy {
            use_rle: false,
            use_delta: false,
            use_dictionary: false,
            original_data: data.to_vec(),
        };
    }

    // Analyze for RLE effectiveness (good for sparse data)
    let zero_runs = count_zero_runs(data);
    let zero_percentage = data.iter().filter(|&&b| b == 0).count() as f32 / data.len() as f32;
    
    if zero_percentage > 0.3 || zero_runs > data.len() / 20 {
        use_rle = true;
    }

    // Analyze for delta coding effectiveness
    let delta_entropy = calculate_delta_entropy(data);
    let original_entropy = calculate_entropy(data);
    
    if delta_entropy < original_entropy * 0.8 {
        use_delta = true;
    }

    // In Max mode, be more aggressive with transforms
    if mode == HlcMode::Max {
        if !use_delta && has_sequential_patterns(data) {
            use_delta = true;
        }
        
        if has_repeating_patterns(data) {
            use_dictionary = true;
        }
    }

    CompressionStrategy {
        use_rle,
        use_delta,
        use_dictionary,
        original_data: data.to_vec(),
    }
}

fn count_zero_runs(data: &[u8]) -> usize {
    let mut runs = 0;
    let mut in_run = false;
    
    for &byte in data {
        if byte == 0 {
            if !in_run {
                runs += 1;
                in_run = true;
            }
        } else {
            in_run = false;
        }
    }
    
    runs
}

fn calculate_entropy(data: &[u8]) -> f32 {
    let mut counts = [0u32; 256];
    for &byte in data {
        counts[byte as usize] += 1;
    }
    
    let len = data.len() as f32;
    let mut entropy = 0.0;
    
    for &count in &counts {
        if count > 0 {
            let p = count as f32 / len;
            entropy -= p * p.log2();
        }
    }
    
    entropy
}

fn calculate_delta_entropy(data: &[u8]) -> f32 {
    if data.len() < 2 {
        return calculate_entropy(data);
    }
    
    let mut deltas = Vec::with_capacity(data.len() - 1);
    for i in 1..data.len() {
        deltas.push(data[i].wrapping_sub(data[i - 1]));
    }
    
    calculate_entropy(&deltas)
}

fn has_sequential_patterns(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }
    
    let mut sequential_count = 0;
    for i in 1..data.len() {
        let diff = data[i].wrapping_sub(data[i - 1]);
        if diff <= 2 {
            sequential_count += 1;
        }
    }
    
    sequential_count > data.len() / 3
}

fn has_repeating_patterns(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    
    // Look for 4-byte patterns that repeat
    let mut pattern_count = 0;
    for i in 0..data.len().saturating_sub(8) {
        let pattern = &data[i..i + 4];
        for j in (i + 4..data.len().saturating_sub(4)).step_by(4) {
            if &data[j..j + 4] == pattern {
                pattern_count += 1;
                break;
            }
        }
    }
    
    pattern_count > data.len() / 64
}