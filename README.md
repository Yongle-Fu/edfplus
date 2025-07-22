> [中文说明请点击这里 (README.zh-CN.md)](README.zh-CN.md)

# EDF+ Library for Rust

A pure Rust implementation of the EDF+ (European Data Format Plus) file read/write library, focused on providing a safe and efficient API.

[![Crates.io](https://img.shields.io/crates/v/edfplus.svg)](https://crates.io/crates/edfplus)
[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg)](https://2986002971.github.io/edfplus/edfplus/)

## 📖 Full Documentation

**[👉 Click to view the complete API documentation and tutorials](https://2986002971.github.io/edfplus/edfplus/)**

The documentation includes:
- 🚀 Quick Start Guide
- 📚 Detailed API Reference
- 💡 Best Practices and Common Pitfalls
- 🧪 Compilable Code Examples
- 🔧 Advanced Usage and Performance Optimization

## Quick Preview

```rust
use edfplus::{EdfReader, EdfWriter, SignalParam};

// Read an EDF+ file
let mut reader = EdfReader::open("data.edf")?;
let samples = reader.read_physical_samples(0, 1000)?;

// Create an EDF+ file
let mut writer = EdfWriter::create("output.edf")?;
writer.add_signal(SignalParam::new_eeg("EEG Fp1", 256))?;
writer.write_samples(&[samples])?;
writer.finalize()?;
```
- For detailed usage of `reader`, see [EdfReader](https://2986002971.github.io/edfplus/edfplus/reader/struct.EdfReader.html)
- For detailed usage of `writer`, see [EdfWriter](https://2986002971.github.io/edfplus/edfplus/writer/struct.EdfWriter.html)
- For writing annotations (event markers), see [add_annotation](https://2986002971.github.io/edfplus/edfplus/writer/struct.EdfWriter.html#method.add_annotation)
- For common sample writing methods and their limitations, see [write_samples](https://2986002971.github.io/edfplus/edfplus/writer/struct.EdfWriter.html#method.write_samples)

## Installation

```toml
[dependencies]
edfplus = "0.1.0"
```

## Features

- ✅ Full EDF+ read/write support
- ✅ Type-safe API design
- ✅ Memory-efficient streaming
- ✅ Rich metadata support
- ✅ Time-accurate annotation system

## Examples

See the [`examples/`](examples/) directory:

```bash
# Generate a test file
cargo run --example generate_test_file

# Basic reading example
cargo run --example basic_example

# Best practices for annotations
cargo run --example annotation_best_practices
```

## ⚠️ Important Notes

- **Annotation Limitations**: Descriptions are limited to 40 characters and must be within the data time range. For details, see [add_annotation](https://2986002971.github.io/edfplus/edfplus/writer/struct.EdfWriter.html#method.add_annotation)
- **Write Limitations**: Backtracking to modify already written data is not supported. For details, see [write_samples](https://2986002971.github.io/edfplus/edfplus/writer/struct.EdfWriter.html#method.write_samples)

## License

This project is licensed under BSD-3-Clause.

## Contributing

Issues and pull requests are welcome!

## Acknowledgements

This library is inspired by the original [EDFlib](https://gitlab.com/Teuniz/EDFlib) C library, but reimplemented with modern Rust best practices.

---

**💡 Tip**: This README provides only a quick overview. For the complete usage guide, API documentation, and best practices, please visit the [online documentation](https://2986002971.github.io/edfplus/edfplus/)