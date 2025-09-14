use hlc::config::{HlcConfig, HlcMode};
use hlc::pipeline;

#[test]
fn round_trip_small_buffer() {
	let input_data = (0..1024u32).flat_map(|x| x.to_le_bytes()).collect::<Vec<_>>();
	let mut reader = std::io::Cursor::new(input_data.clone());
	let mut compressed = Vec::new();
	let cfg = HlcConfig { mode: HlcMode::Balanced, ..Default::default() };
	let _ = pipeline::compress(&mut reader, &mut compressed, &cfg).unwrap();

	let mut comp_reader = std::io::Cursor::new(compressed);
	let mut restored = Vec::new();
	pipeline::decompress(&mut comp_reader, &mut restored, cfg.threads).unwrap();
	assert_eq!(restored, input_data);
}