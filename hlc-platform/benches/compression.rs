use criterion::{criterion_group, criterion_main, Criterion, black_box};
use hlc::{config::HlcConfig, pipeline};

fn bench_compress(c: &mut Criterion) {
    let data = vec![0u8; 2 * 1024 * 1024]; // 2MB of zeros for RLE
    c.bench_function("compress_2mb_zeros", |b| {
        b.iter(|| {
            let mut reader = std::io::Cursor::new(data.clone());
            let mut out = Vec::new();
            let _ = pipeline::compress(&mut reader, &mut out, &HlcConfig::default()).unwrap();
            black_box(out);
        })
    });
}

criterion_group!(benches, bench_compress);
criterion_main!(benches);

