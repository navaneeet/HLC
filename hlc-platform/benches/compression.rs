use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use hlc::config::{HlcConfig, HlcMode};
use hlc::pipeline;

fn bench_compress(c: &mut Criterion) {
	let data = vec![0u8; 8 * 1024 * 1024];
	let config = HlcConfig { mode: HlcMode::Balanced, ..Default::default() };
	let mut group = c.benchmark_group("compression");
	group.throughput(Throughput::Bytes(data.len() as u64));
	group.bench_function("compress_zeroes", |b| {
		b.iter(|| {
			let mut input = std::io::Cursor::new(&data);
			let mut output = Vec::new();
			let _ = pipeline::compress(&mut input, &mut output, &config).unwrap();
		});
	});
	group.finish();
}

criterion_group!(benches, bench_compress);
criterion_main!(benches);