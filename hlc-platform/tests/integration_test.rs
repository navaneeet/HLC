use hlc::{config::HlcConfig, pipeline};

#[test]
fn round_trip_small_buffer() {
    let input = b"The quick brown fox jumps over the lazy dog\0\0\0\0".to_vec();
    let mut reader = std::io::Cursor::new(input.clone());
    let mut compressed = Vec::new();
    let config = HlcConfig::default();
    let _ = pipeline::compress(&mut reader, &mut compressed, &config).expect("compress");

    let mut decompressed = Vec::new();
    let mut comp_reader = std::io::Cursor::new(compressed);
    pipeline::decompress(&mut comp_reader, &mut decompressed, config.threads).expect("decompress");
    assert_eq!(input, decompressed);
}

