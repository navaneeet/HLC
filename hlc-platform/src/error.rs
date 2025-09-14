use thiserror::Error;

#[derive(Error, Debug)]
pub enum HlcError {
	#[error("I/O error: {0}")]
	Io(#[from] std::io::Error),

	#[error("Data integrity check failed: checksum mismatch")]
	ChecksumMismatch,

	#[error("Invalid HLC container format: {0}")]
	InvalidFormat(String),

	#[error("Compression failed: {0}")]
	CompressionError(String),

	#[error("Decompression failed: {0}")]
	DecompressionError(String),

	#[error("Configuration error: {0}")]
	ConfigError(String),
}