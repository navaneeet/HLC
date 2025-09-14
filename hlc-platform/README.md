# HLC - Hybrid Lossless Compression

Build:

```bash
cargo build --release
```

CLI:

```bash
./target/release/hlc compress -i INPUT -o OUTPUT.hlc --mode balanced
./target/release/hlc decompress -i OUTPUT.hlc -o RESTORED
```

Library usage example in `examples/usage_sdk.rs`.