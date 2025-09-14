//! HLC (Hybrid Lossless Compression) Platform - Command Line Interface
//! 
//! This is the main entry point for the HLC command-line tool.
//! 
//! For library usage, see the `hlc` crate documentation.

use hlc::cli;
use std::process;

fn main() {
    // Initialize logger for monitoring and debugging
    let _ = env_logger::try_init(); // Use try_init to avoid panicking
    
    // Run the CLI application
    if let Err(e) = cli::run() {
        eprintln!("Error: {}", e);
        
        // Print additional context for certain error types
        match &e {
            hlc::HlcError::Io(io_err) => {
                eprintln!("I/O operation failed. Please check file permissions and disk space.");
                if io_err.kind() == std::io::ErrorKind::NotFound {
                    eprintln!("Make sure the input file exists and the path is correct.");
                }
            },
            hlc::HlcError::ChecksumMismatch => {
                eprintln!("Data integrity check failed. The file may be corrupted.");
                eprintln!("Try using a different checksum type or re-compress the original file.");
            },
            hlc::HlcError::InvalidFormat(_) => {
                eprintln!("The input file does not appear to be a valid HLC archive.");
                eprintln!("Make sure you're trying to decompress a file created with HLC.");
            },
            hlc::HlcError::ThreadPoolError(_) => {
                eprintln!("Failed to initialize thread pool. Try reducing the number of threads.");
            },
            _ => {
                eprintln!("Run with RUST_LOG=debug for more detailed error information.");
            }
        }
        
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    // Integration tests for the main binary would go here
    // These would test the full CLI workflow
}