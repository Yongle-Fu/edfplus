//! # EDF+ Library for Rust
//! 
//! A pure Rust library for reading and writing EDF+ (European Data Format Plus) files.
//! This library focuses specifically on EDF+ format and provides a safe, efficient API.
//!
//! ## Quick Start
//!
//! ### Reading an EDF+ file
//!
//! ```rust
//! use edfplus::{EdfReader, EdfWriter, SignalParam, Result};
//! # use std::fs;
//!
//! fn main() -> Result<()> {
//!     # // Create a test file first
//!     # let mut writer = EdfWriter::create("test_data.edf")?;
//!     # writer.set_patient_info("P001", "M", "01-JAN-1990", "Test Patient")?;
//!     # let signal = SignalParam {
//!     #     label: "EEG".to_string(),
//!     #     samples_in_file: 0,
//!     #     physical_max: 100.0,
//!     #     physical_min: -100.0,
//!     #     digital_max: 32767,
//!     #     digital_min: -32768,
//!     #     samples_per_record: 256,
//!     #     physical_dimension: "uV".to_string(),
//!     #     prefilter: "HP:0.1Hz".to_string(),
//!     #     transducer: "AgAgCl".to_string(),
//!     # };
//!     # writer.add_signal(signal)?;
//!     # let samples = vec![10.0; 256];
//!     # for _ in 0..10 { writer.write_samples(&[samples.clone()])?; }
//!     # writer.finalize()?;
//! 
//!     // Open an EDF+ file
//!     let mut reader = EdfReader::open("test_data.edf")?;
//!     
//!     // Get file information
//!     let header = reader.header();
//!     println!("Number of signals: {}", header.signals.len());
//!     println!("File duration: {:.2} seconds", 
//!         header.file_duration as f64 / 10_000_000.0);
//!     
//!     // Read first 1000 samples from signal 0
//!     if !header.signals.is_empty() {
//!         let samples = reader.read_physical_samples(0, 1000)?;
//!         println!("Read {} samples", samples.len());
//!     }
//!     
//!     # // Cleanup
//!     # drop(reader);
//!     # fs::remove_file("test_data.edf").ok();
//!     Ok(())
//! }
//! ```
//!
//! ### Creating an EDF+ file
//!
//! ```rust
//! use edfplus::{EdfWriter, SignalParam, Result};
//! # use std::fs;
//!
//! fn main() -> Result<()> {
//!     // Create a writer
//!     let mut writer = EdfWriter::create("test_output.edf")?;
//!     
//!     // Set patient information
//!     writer.set_patient_info("P001", "M", "01-JAN-1990", "Patient Name")?;
//!     
//!     // Define signal parameters
//!     let signal = SignalParam {
//!         label: "EEG Fp1".to_string(),
//!         samples_in_file: 0,  // Will be calculated automatically
//!         physical_max: 200.0,
//!         physical_min: -200.0,
//!         digital_max: 32767,
//!         digital_min: -32768,
//!         samples_per_record: 256,  // Sample rate
//!         physical_dimension: "uV".to_string(),
//!         prefilter: "HP:0.1Hz LP:70Hz".to_string(),
//!         transducer: "AgAgCl cup electrodes".to_string(),
//!     };
//!     
//!     // Add the signal
//!     writer.add_signal(signal)?;
//!     
//!     // Generate and write data
//!     let mut samples = Vec::new();
//!     for i in 0..256 {
//!         let t = i as f64 / 256.0;
//!         let value = 50.0 * (2.0 * std::f64::consts::PI * 10.0 * t).sin();
//!         samples.push(value);
//!     }
//!     
//!     writer.write_samples(&[samples])?;
//!     writer.finalize()?;
//!     
//!     # // Cleanup
//!     # fs::remove_file("test_output.edf").ok();
//!     Ok(())
//! }
//! ```
//! ### Adds an annotation/event to the EDF+ file
//! 
//! **⚠️ CRITICAL TIMING CONSTRAINT**
//! 
//! Annotations are only saved when their onset time falls within **future data records**.
//! Once a data record is written with `write_samples()`, no new annotations can be 
//! added to that time period.
//! 
//! **Timing Rules:**
//! - Add annotations **BEFORE** writing the data records that cover their time range
//! - Annotations with `onset_seconds` in already-written time periods will be **silently lost**
//! - This is due to the sequential write architecture - no backtracking is possible
//! 
//! ##### Arguments
//! 
//! * `onset_seconds` - Time when the event occurred (seconds since recording start)
//! * `duration_seconds` - Duration of the event in seconds (None for instantaneous events)  
//! * `description` - UTF-8 text describing the event (max 40 chars effective)
//! 
//! ##### Description Length Limit
//! 
//! **Warning**: Annotation descriptions are subject to EDF+ format constraints:
//! - Maximum effective length is **40 characters** in the final TAL (Time-stamped Annotations Lists) data
//! - Longer descriptions will be **automatically truncated** during file writing
//! - UTF-8 multi-byte characters may be truncated at byte boundaries, potentially corrupting the text
//! - This limit is enforced by the EDF+ standard and matches edflib behavior
//! 
//! ```rust
//! // Write 5 seconds of data (5 records)
//! # use edfplus::{EdfWriter, SignalParam, Result};
//! # fn main() -> Result<()> {
//! let mut writer = EdfWriter::create("annotations.edf")?;
//! # let signal = SignalParam {
//! #     label: "EEG".to_string(),
//! #     samples_in_file: 0,
//! #     physical_max: 100.0,
//! #     physical_min: -100.0,
//! #     digital_max: 32767,
//! #     digital_min: -32768,
//! #     samples_per_record: 256,
//! #     physical_dimension: "uV".to_string(),
//! #     prefilter: "".to_string(),
//! #     transducer: "".to_string(),
//! # };
//! # writer.add_signal(signal)?;
//! 
//! // ✅ Good - within file duration [0.0, 5.0)
//! writer.add_annotation(2.5, None, "Valid event")?;
//! writer.add_annotation(4.999, None, "Last moment")?;
//! 
//! // ❌ Lost - outside file duration
//! writer.add_annotation(5.0, None, "Will be discarded")?;
//! writer.add_annotation(6.0, None, "Also discarded")?;
//! 
//! for i in 0..5 {
//!     let samples = vec![0.0; 256];
//!     writer.write_samples(&[samples])?;
//! }
//! # std::fs::remove_file("annotations.edf").ok();
//! # Ok(())
//! # }
//! ```
//! 
//! ## Working with Signal Data
//!
//! ### Physical vs Digital Values
//!
//! EDF+ stores data as 16-bit integers but represents real-world measurements.
//! The library automatically handles conversion between digital and physical values:
//!
//! ```rust
//! use edfplus::SignalParam;
//!
//! let signal = SignalParam {
//!     label: "Test Signal".to_string(),
//!     samples_in_file: 1000,
//!     physical_max: 100.0,   // +100 µV
//!     physical_min: -100.0,  // -100 µV  
//!     digital_max: 32767,    // +32767 (16-bit max)
//!     digital_min: -32768,   // -32768 (16-bit min)
//!     samples_per_record: 256,
//!     physical_dimension: "uV".to_string(),
//!     prefilter: "".to_string(),
//!     transducer: "".to_string(),
//! };
//!
//! // Convert digital to physical
//! let digital_value = 16384;  // Half of max digital value
//! let physical_value = signal.to_physical(digital_value);
//! assert!((physical_value - 50.0).abs() < 0.1); // Should be ~50 µV
//!
//! // Convert physical to digital  
//! let physical_input = 25.0;  // 25 µV
//! let digital_output = signal.to_digital(physical_input);
//! assert!((digital_output - 8192).abs() <= 1); // Should be ~8192
//! ```



pub mod error;
pub mod types;
pub mod utils;
pub mod reader;
pub mod writer; // 新增

#[doc(hidden)]
pub mod doctest_utils; // For internal doctest support

// Re-export main types for convenience
pub use error::{EdfError, Result};
pub use types::{EdfHeader, SignalParam, Annotation};
pub use reader::EdfReader;
pub use writer::EdfWriter; // 新增

// Important constants
pub const EDFLIB_TIME_DIMENSION: i64 = 10_000_000; // 100 nanoseconds unit
pub const EDFLIB_MAXSIGNALS: usize = 4096;
pub const EDFLIB_MAX_ANNOTATION_LEN: usize = 512;

/// Library version
/// 
/// Returns the current version of the edfplus library.
///
/// # Examples
///
/// ```rust
/// use edfplus;
///
/// let version = edfplus::version();
/// assert!(!version.is_empty());
/// assert!(version.contains('.'));
/// println!("EDF+ library version: {}", version);
/// ```
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
