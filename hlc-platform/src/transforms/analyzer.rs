use crate::config::HlcMode;

pub struct CompressionStrategy {
    pub use_rle: bool,
    pub use_delta: bool,
    pub original_data: Vec<u8>,
}

pub fn analyze_chunk(data: &[u8], mode: HlcMode) -> CompressionStrategy {
    let mut use_rle = false;
    let mut use_delta = false;

    if !data.is_empty() {
        let zero_count = data.iter().filter(|&&b| b == 0).count() as f32;
        let zero_percentage = zero_count / data.len() as f32;
        if zero_percentage > 0.4 {
            use_rle = true;
        }
    }
    
    if mode == HlcMode::Max {
        use_delta = true;
    }

    CompressionStrategy {
        use_rle,
        use_delta,
        original_data: data.to_vec(),
    }
}

