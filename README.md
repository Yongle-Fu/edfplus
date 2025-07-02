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

### 创建EDF+文件（多通道示例）

```rust
use edfplus::{EdfWriter, SignalParam, Result};

fn main() -> Result<()> {
    // 创建写入器
    let mut writer = EdfWriter::create("multi_channel_eeg.edf")?;
    
    // 设置患者信息
    writer.set_patient_info("P001", "M", "01-JAN-1990", "Patient Name")?;
    
    // 定义多个EEG通道
    let channels = vec![
        ("EEG Fp1", -200.0, 200.0),  // 前额左
        ("EEG Fp2", -200.0, 200.0),  // 前额右  
        ("EEG C3", -200.0, 200.0),   // 中央左
        ("EEG C4", -200.0, 200.0),   // 中央右
        ("EEG O1", -200.0, 200.0),   // 枕部左
        ("EEG O2", -200.0, 200.0),   // 枕部右
        ("EOG", -500.0, 500.0),      // 眼电图
        ("EMG", -100.0, 100.0),      // 肌电图
    ];
    
    // 为每个通道添加信号参数
    for (label, phys_min, phys_max) in &channels {
        let signal = SignalParam {
            label: label.to_string(),
            samples_in_file: 0,
            physical_max: *phys_max,
            physical_min: *phys_min,
            digital_max: 32767,
            digital_min: -32768,
            samples_per_record: 256,  // 256 Hz采样率
            physical_dimension: "uV".to_string(),
            prefilter: "HP:0.1Hz LP:70Hz".to_string(),
            transducer: "AgAgCl cup electrodes".to_string(),
        };
        writer.add_signal(signal)?;
    }
    
    // 模拟记录10秒的数据（10个数据记录，每个1秒）
    for record in 0..10 {
        let mut all_samples = Vec::new();
        
        // 为每个通道生成一秒的数据（256个样本）
        for (chan_idx, (label, _, _)) in channels.iter().enumerate() {
            let mut channel_samples = Vec::new();
            
            for i in 0..256 {
                let t = (record as f64) + (i as f64 / 256.0);
                
                // 根据通道类型生成不同的信号
                let value = match label {
                    label if label.starts_with("EEG") => {
                        // EEG信号：多个频率成分的组合
                        let alpha = 20.0 * (2.0 * std::f64::consts::PI * 10.0 * t).sin();
                        let beta = 5.0 * (2.0 * std::f64::consts::PI * 20.0 * t).sin();
                        let noise = fastrand::f64() * 10.0 - 5.0;
                        alpha + beta + noise
                    },
                    "EOG" => {
                        // 眼电图：低频眨眼信号
                        if t % 3.0 < 0.1 { 100.0 } else { 0.0 } + fastrand::f64() * 20.0 - 10.0
                    },
                    "EMG" => {
                        // 肌电图：高频肌肉活动
                        (fastrand::f64() - 0.5) * 50.0 * (1.0 + (t % 5.0 / 5.0))
                    },
                    _ => 0.0
                };
                
                channel_samples.push(value);
            }
            all_samples.push(channel_samples);
        }
        
        // 写入所有通道的数据
        writer.write_samples(&all_samples)?;
        
        // 在第3秒和第7秒添加注释
        if record == 3 {
            writer.add_annotation(0.5, "开始集中注意力任务")?;
        }
        if record == 7 {
            writer.add_annotation(0.2, "任务结束，开始休息")?;
        }
    }
    
    writer.finalize()?;
    
    println!("已创建多通道EEG文件 multi_channel_eeg.edf");
    println!("包含 {} 个通道，每个通道记录10秒数据", channels.len());
    
    Ok(())
}
```

这个示例展示了如何创建一个包含8个通道的EEG记录文件：
- **6个EEG通道**：Fp1/Fp2（前额）、C3/C4（中央）、O1/O2（枕部）
- **1个EOG通道**：眼电图，用于检测眨眼和眼动
- **1个EMG通道**：肌电图，用于监测肌肉活动

每个通道都有适合其信号类型的物理范围和模拟信号特征。在实际应用中，你可以：
- 调整采样率（`samples_per_record`）适应不同需求
- 设置合适的物理范围（`physical_min`/`physical_max`）
- 添加滤波器信息（`prefilter`）描述信号处理
- 同时记录多种生理信号（EEG、EOG、EMG等）

## 添加注释/事件标记

EDF+支持时间标记的注释来记录事件、阶段变化等重要信息：

```rust
use edfplus::{EdfWriter, SignalParam, Result};

fn main() -> Result<()> {
    let mut writer = EdfWriter::create("sleep_study.edf")?;
    writer.set_patient_info("S001", "F", "15-MAR-1980", "Sleep Study")?;
    
    // 添加EEG信号
    let eeg_signal = SignalParam {
        label: "C3-A2".to_string(),
        samples_in_file: 0,
        physical_max: 100.0,
        physical_min: -100.0,
        digital_max: 32767,
        digital_min: -32768,
        samples_per_record: 100,  // 100 Hz
        physical_dimension: "uV".to_string(),
        prefilter: "0.1-35Hz".to_string(),
        transducer: "AgAgCl".to_string(),
    };
    writer.add_signal(eeg_signal)?;
    
    // 添加睡眠研究注释 - 注意：必须在写入数据前添加
    writer.add_annotation(300.0, None, "Lights out")?;                    // 5分钟
    writer.add_annotation(480.0, None, "Sleep onset")?;                   // 8分钟  
    writer.add_annotation(600.0, Some(1200.0), "Stage N2")?;              // 10-30分钟
    writer.add_annotation(900.0, None, "Sleep spindle")?;                 // 15分钟
    writer.add_annotation(1200.0, Some(300.0), "REM episode")?;           // 20-25分钟
    writer.add_annotation(1790.0, None, "Wake up")?;                      // 29:50
    
    // ⚠️ 重要：在添加注释后写入数据以建立时间范围
    let recording_duration_seconds = 1800;  // 30分钟
    for second in 0..recording_duration_seconds {
        let mut samples = Vec::with_capacity(100);
        for sample_idx in 0..100 {
            let t = second as f64 + (sample_idx as f64 / 100.0);
            let eeg_value = 20.0 * (2.0 * std::f64::consts::PI * 10.0 * t).sin();
            samples.push(eeg_value);
        }
        writer.write_samples(&[samples])?;
    }
    
    writer.finalize()?;
    Ok(())
}
```

## ⚠️ 注释使用的重要限制

使用注释功能时，请注意以下关键限制，以免数据丢失：

### 1. 描述长度限制

**注释描述最多只能存储40个字符**，超出部分会被自动截断：

```rust
// ✅ 正确 - 在40字符限制内
writer.add_annotation(1.0, None, "Sleep stage N2")?;

// ⚠️ 警告 - 会被截断
writer.add_annotation(2.0, None, "This is a very long annotation that exceeds the EDF+ limit")?;
// 结果: "This is a very long annotation descripti"

// 💡 建议 - 使用简洁的描述
writer.add_annotation(3.0, None, "REM burst")?;
writer.add_annotation(4.0, None, "K-complex")?;
writer.add_annotation(5.0, None, "Artifact")?;
```

### 2. 时间范围约束

**最关键的限制**：注释只有在其时间戳落在已写入的数据记录范围内时才会被保存：

```rust
// ❌ 错误的顺序 - 注释会丢失
// 只写入3秒的数据 - 时间范围 [0.0, 3.0)
for second in 0..3 {
    let samples = vec![0.0; 256];
    writer.write_samples(&[samples])?;
}

// 然后添加注释 - 但这些时间超出了数据范围
writer.add_annotation(5.0, None, "Event at 5s")?;    // ❌ 会被丢失
writer.add_annotation(10.0, None, "Event at 10s")?;  // ❌ 会被丢失
// 结果：所有注释都会丢失！

// ✅ 正确的顺序
// 1. 先添加注释
writer.add_annotation(5.0, None, "Event at 5s")?;    // 预计在5秒时发生
writer.add_annotation(10.0, None, "Event at 10s")?;  // 预计在10秒时发生
writer.add_annotation(14.999, None, "Near end")?;    // 预计在14.999秒时发生

// 2. 然后写入足够的数据覆盖这些时间点
for second in 0..15 {  // 15秒数据，范围 [0.0, 15.0)
    let samples = vec![0.0; 256];
    writer.write_samples(&[samples])?;
}
// 结果：前3个注释都会被保存！

// ❌ 这个注释会被丢失，因为添加时数据范围已确定
writer.add_annotation(16.0, None, "Too late")?;  // ❌ 超出范围
```

### 3. 最佳实践

为避免数据丢失，请遵循以下最佳实践：

```rust
// 1. 📝 使用简洁的ASCII描述（≤40字符）
writer.add_annotation(1.0, None, "N1")?;           // 优于 "Sleep Stage N1 Beginning"
writer.add_annotation(2.0, None, "Spindle")?;      // 优于 "Sleep Spindle Activity Detected"
writer.add_annotation(3.0, None, "REM")?;          // 优于 "Rapid Eye Movement Sleep Phase"

// 2. 📊 规划注释时间，然后写入覆盖这些时间的数据
// 先添加所有预期的注释
writer.add_annotation(3600.0, None, "1h mark")?;   // 1小时标记
writer.add_annotation(7200.0, None, "2h mark")?;   // 2小时标记

// 然后写入足够时长的数据
let study_duration_hours = 8.0;
let total_seconds = (study_duration_hours * 3600.0) as usize;
for second in 0..total_seconds {
    // ... 写入数据 ...
}

// 3. 🕒 验证注释时间在预期数据范围内
fn add_safe_annotation(writer: &mut EdfWriter, time: f64, desc: &str, max_time: f64) -> Result<()> {
    if time >= max_time {
        eprintln!("警告: 注释时间 {:.1}s 超出预期文件范围 {:.1}s，请调整", time, max_time);
        return Ok(());
    }
    if desc.len() > 40 {
        eprintln!("警告: 描述 '{}' 超过40字符，将被截断", desc);
    }
    writer.add_annotation(time, None, desc)
}
```

### 4. UTF-8字符注意事项

由于40字符限制，多字节UTF-8字符可能被不当截断：

```rust
// ⚠️ 可能导致无效UTF-8
writer.add_annotation(1.0, None, "测试中文字符和emoji🧠很长的描述文本")?;
// 可能被截断为: "测试中文字符和emoji🧠很长�" (无效UTF-8)

// ✅ 建议使用ASCII字符
writer.add_annotation(1.0, None, "Chinese text test")?;
writer.add_annotation(2.0, None, "Event with emoji")?;
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
- `annotation_best_practices.rs` - **注释使用最佳实践演示**

运行示例：

```bash
# 生成测试文件
cargo run --example generate_test_file

# 运行基本读取示例
cargo run --example basic_example

# 运行详细读取示例
cargo run --example detailed_read_example

# 学习注释使用的正确方法（重要！）
cargo run --example annotation_best_practices
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

本库参考了原始的[EDFlib](https://gitlab.com/Teuniz/EDFlib) C库的设计思想，但采用了现代Rust的最佳实践重新实现。
