use hlc::{config::HlcConfig, pipeline};
use std::io::Cursor;

fn main() {
    let input = b"Hello Hello Hello\0\0\0\0\0 world!".to_vec();
    let mut reader = Cursor::new(input.clone());
    let mut compressed = Vec::new();
    let config = HlcConfig::default();
    let stats = pipeline::compress(&mut reader, &mut compressed, &config).expect("compress");
    println!("Compressed {} -> {} bytes (ratio {:.2}x)", stats.original_size, stats.compressed_size, stats.ratio);

    let mut decompressed = Vec::new();
    let mut comp_reader = Cursor::new(compressed);
    pipeline::decompress(&mut comp_reader, &mut decompressed, config.threads).expect("decompress");
    assert_eq!(input, decompressed);
    println!("Round-trip OK");
}

