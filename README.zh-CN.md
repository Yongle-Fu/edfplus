> [English README here (README.md)](README.md)

# EDF+ Library for Rust

一个纯Rust实现的EDF+（欧洲数据格式增强版）文件读写库，专注于提供安全、高效的API。

[![Crates.io](https://img.shields.io/crates/v/edfplus.svg)](https://crates.io/crates/edfplus)
[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg)](https://2986002971.github.io/edfplus/edfplus/)

## 📖 完整文档

**[👉 点击查看完整API文档和教程](https://2986002971.github.io/edfplus/edfplus/)**

文档包含：
- 🚀 快速开始指南
- 📚 详细API参考
- 💡 最佳实践和常见陷阱
- 🧪 经过编译验证的代码示例
- 🔧 高级用法和性能优化

## 快速预览

```rust
use edfplus::{EdfReader, EdfWriter, SignalParam};

// 读取EDF+文件
let mut reader = EdfReader::open("data.edf")?;
let samples = reader.read_physical_samples(0, 1000)?;

// 创建EDF+文件
let mut writer = EdfWriter::create("output.edf")?;
writer.add_signal(SignalParam::new_eeg("EEG Fp1", 256))?;
writer.write_samples(&[samples])?;
writer.finalize()?;
```
- 使用reader查看文件的详细写法请参考[EdfReader](https://2986002971.github.io/edfplus/edfplus/reader/struct.EdfReader.html)
- 使用writer写入文件的详细写法请参考[EdfWriter](https://2986002971.github.io/edfplus/edfplus/writer/struct.EdfWriter.html)
- 写入注释（事件标记）请参考[add_annotation](https://2986002971.github.io/edfplus/edfplus/writer/struct.EdfWriter.html#method.add_annotation)
- 常用的写入样本方法与其限制请参考[write_samples](https://2986002971.github.io/edfplus/edfplus/writer/struct.EdfWriter.html#method.write_samples)

## 安装

```toml
[dependencies]
edfplus = "0.1.0"
```

## 特性

- ✅ 完整的EDF+读写支持
- ✅ 类型安全的API设计  
- ✅ 内存高效的流式处理
- ✅ 丰富的元数据支持
- ✅ 时间精确的注释系统

## 示例

查看 [`examples/`](examples/) 目录：

```bash
# 生成测试文件
cargo run --example generate_test_file

# 基本读取示例
cargo run --example basic_example

# 注释使用最佳实践
cargo run --example annotation_best_practices
```

## ⚠️ 重要提醒

- **注释限制**: 描述最多40字符，且必须在数据时间范围内，具体限制请参考[add_annotation](https://2986002971.github.io/edfplus/edfplus/writer/struct.EdfWriter.html#method.add_annotation)
- **写入限制**: 不支持回溯修改已写入的数据，原因请参考[write_samples](https://2986002971.github.io/edfplus/edfplus/writer/struct.EdfWriter.html#method.write_samples)


## 许可证

本项目采用 BSD-3-Clause 许可证。

## 贡献

欢迎提交issue和pull request！

## 致谢

本库参考了原始的[EDFlib](https://gitlab.com/Teuniz/EDFlib) C库的设计思想，但采用了现代Rust的最佳实践重新实现。

---

**💡 提示**: 本README仅提供快速概览。完整的使用指南、API文档和最佳实践请访问[在线文档](https://2986002971.github.io/edfplus/edfplus/)。