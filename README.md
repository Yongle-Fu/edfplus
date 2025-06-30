# EDF+ Library for Rust

一个纯Rust实现的EDF+（欧洲数据格式增强版）文件读写库。本库专注于EDF+格式，提供安全、高效的API。

## 特性

- ✅ **读取EDF+文件** - 完整的头部信息和信号数据读取
- ✅ **写入EDF+文件** - 创建标准的EDF+文件
- ✅ **类型安全** - 利用Rust的类型系统防止常见错误
- ✅ **内存安全** - 无缓冲区溢出或内存泄漏
- ✅ **高效处理** - 支持大文件的流式读取
- ✅ **完整的元数据** - 患者信息、设备信息等
- ✅ **信号处理** - 物理值与数字值的自动转换
- ✅ **文件定位** - 支持随机访问和定位

## 快速开始

将以下内容添加到你的 `Cargo.toml`:

```toml
[dependencies]
edfplus = "0.1.0"
```

### 读取EDF+文件

```rust
use edfplus::{EdfReader, Result};

fn main() -> Result<()> {
    // 打开EDF+文件
    let mut reader = EdfReader::open("data.edf")?;
    
    // 获取文件信息
    let header = reader.header();
    println!("信号数量: {}", header.signals.len());
    println!("文件时长: {:.2} 秒", header.file_duration as f64 / 10_000_000.0);
    
    // 读取第一个信号的前1000个样本
    let samples = reader.read_physical_samples(0, 1000)?;
    println!("读取了 {} 个样本", samples.len());
    
    // 获取信号信息
    for (i, signal) in header.signals.iter().enumerate() {
        println!("信号 {}: {} ({})", i, signal.label, signal.physical_dimension);
        println!("  范围: {} - {}", signal.physical_min, signal.physical_max);
    }
    
    Ok(())
}
```

### 创建EDF+文件

```rust
use edfplus::{EdfWriter, SignalParam, Result};

fn main() -> Result<()> {
    // 创建写入器
    let mut writer = EdfWriter::create("output.edf")?;
    
    // 设置患者信息
    writer.set_patient_info("P001", "M", "01-JAN-1990", "Patient Name")?;
    
    // 定义信号参数
    let signal = SignalParam {
        label: "EEG Fp1".to_string(),
        samples_in_file: 0,  // 会自动计算
        physical_max: 200.0,
        physical_min: -200.0,
        digital_max: 32767,
        digital_min: -32768,
        samples_per_record: 256,  // 采样率
        physical_dimension: "uV".to_string(),
        prefilter: "HP:0.1Hz LP:70Hz".to_string(),
        transducer: "AgAgCl cup electrodes".to_string(),
    };
    
    // 添加信号
    writer.add_signal(signal)?;
    
    // 生成并写入数据
    let mut samples = Vec::new();
    for i in 0..256 {
        let t = i as f64 / 256.0;
        let value = 50.0 * (2.0 * std::f64::consts::PI * 10.0 * t).sin();
        samples.push(value);
    }
    
    writer.write_samples(&[samples])?;
    writer.finalize()?;
    
    Ok(())
}
```

## API 文档

### 核心类型

- `EdfReader` - 用于读取EDF+文件
- `EdfWriter` - 用于创建EDF+文件  
- `EdfHeader` - 文件头部信息
- `SignalParam` - 信号参数
- `Annotation` - 注释/事件信息

### 错误处理

库使用 `Result<T, EdfError>` 类型进行错误处理：

```rust
match reader.read_physical_samples(0, 100) {
    Ok(samples) => {
        // 处理样本数据
    }
    Err(EdfError::InvalidSignalIndex(idx)) => {
        println!("无效的信号索引: {}", idx);
    }
    Err(e) => {
        println!("其他错误: {}", e);
    }
}
```

## 示例

查看 `examples/` 目录获取更多示例：

- `basic_example.rs` - 基本文件读取
- `detailed_read_example.rs` - 详细的读取示例
- `generate_test_file.rs` - 创建测试文件

运行示例：

```bash
# 生成测试文件
cargo run --example generate_test_file

# 运行基本读取示例
cargo run --example basic_example

# 运行详细读取示例
cargo run --example detailed_read_example
```

## EDF+ 格式说明

EDF+（European Data Format Plus）是一种用于存储生物医学信号的标准格式，广泛应用于：

- 脑电图（EEG）
- 心电图（ECG）
- 肌电图（EMG）
- 睡眠研究
- 其他生理信号记录

### 关键概念

- **物理值 vs 数字值**: EDF+存储16位整数，通过线性变换转换为实际的物理测量值
- **数据记录**: 文件被分割为固定时间间隔的记录
- **注释**: EDF+支持时间标记的事件和注释

## 性能

- **内存效率**: 支持流式读取，内存使用量与文件大小无关
- **速度**: 针对大文件优化的读取性能
- **并发安全**: 结构体设计支持多线程访问（读取器除外）

## 兼容性

- **Rust版本**: 需要 Rust 1.70+
- **平台**: 支持所有Rust支持的平台
- **EDF版本**: 专注于EDF+格式，不支持原始EDF格式

## 许可证

本项目采用 BSD-3-Clause 许可证。

## 贡献

欢迎提交issue和pull request！

## 致谢

本库参考了原始的EDFlib C库的设计思想，但采用了现代Rust的最佳实践重新实现。
