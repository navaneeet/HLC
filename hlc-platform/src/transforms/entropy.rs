use crate::error::HlcError;

const ZSTD_LEVEL: i32 = 5;

pub fn encode(data: &[u8]) -> Result<Vec<u8>, HlcError> {
    zstd::encode_all(data, ZSTD_LEVEL)
        .map_err(|e| HlcError::CompressionError(e.to_string()))
}

pub fn decode(data: &[u8]) -> Result<Vec<u8>, HlcError> {
    zstd::decode_all(data)
        .map_err(|e| HlcError::DecompressionError(e.to_string()))
}

