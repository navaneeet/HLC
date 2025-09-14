use hlc::config::{HlcConfig, HlcMode};
use hlc::pipeline;

fn main() {
	let data = b"hello hello hello hello".to_vec();
	let mut input = std::io::Cursor::new(data);
	let mut compressed = Vec::new();
	let cfg = HlcConfig { mode: HlcMode::Balanced, ..Default::default() };
	let stats = pipeline::compress(&mut input, &mut compressed, &cfg).unwrap();
	println!("compressed {} -> {}", stats.original_size, stats.compressed_size);

	let mut comp_cur = std::io::Cursor::new(compressed);
	let mut restored = Vec::new();
	pipeline::decompress(&mut comp_cur, &mut restored, cfg.threads).unwrap();
	println!("restored {} bytes", restored.len());
}