//! # EDF+ Library for Rust
//! 
//! A pure Rust library for reading and writing EDF+ (European Data Format Plus) files.
//! This library focuses specifically on EDF+ format and provides a safe, efficient API.

pub mod error;
pub mod types;
pub mod utils;
pub mod reader;

// Re-export main types for convenience
pub use error::{EdfError, Result};
pub use types::{EdfHeader, SignalParam, Annotation, FileType};
pub use reader::EdfReader;

// Important constants
pub const EDFLIB_TIME_DIMENSION: i64 = 10_000_000; // 100 nanoseconds unit
pub const EDFLIB_MAXSIGNALS: usize = 4096;
pub const EDFLIB_MAX_ANNOTATION_LEN: usize = 512;

/// Library version
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }
}
