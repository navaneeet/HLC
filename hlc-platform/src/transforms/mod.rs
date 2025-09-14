pub mod analyzer;
pub mod delta;
pub mod dictionary;
pub mod entropy;
pub mod rle;

pub use analyzer::{analyze_chunk, CompressionStrategy};
pub use entropy::{encode_fast, encode_balanced, encode_max, estimate_compression_ratio};

// Re-export specific functions to avoid naming conflicts
pub use delta::{encode as delta_encode, decode as delta_decode, encode_advanced as delta_encode_advanced, decode_advanced as delta_decode_advanced};
pub use rle::{encode as rle_encode, decode as rle_decode};
pub use dictionary::{encode as dict_encode, decode as dict_decode, train_dictionary, Dictionary};