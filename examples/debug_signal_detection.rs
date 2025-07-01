use edfplus::{EdfWriter, EdfReader, SignalParam};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 注释信号识别调试 ===");
    
    // 创建一个测试文件
    let filename = "debug_signal_detection.edf";
    {
        let mut writer = EdfWriter::create(filename)?;
        writer.set_patient_info("P001", "M", "01-JAN-1990", "Debug Patient")?;
        
        let signal = SignalParam {
            label: "Test Signal".to_string(),
            samples_in_file: 0,
            physical_max: 100.0,
            physical_min: -100.0,
            digital_max: 32767,
            digital_min: -32768,
            samples_per_record: 10,
            physical_dimension: "uV".to_string(),
            prefilter: "None".to_string(),
            transducer: "Test".to_string(),
        };
        
        writer.add_signal(signal)?;
        writer.add_annotation(0.5, None, "Test Event")?;
        
        // 写入1秒的数据
        let samples: Vec<f64> = (0..10).map(|j| j as f64).collect();
        writer.write_samples(&[samples])?;
        writer.finalize()?;
    }
    
    println!("测试文件创建完成");
    
    // 手动检查头部信息
    let mut file = File::open(filename)?;
    
    // 读取主头部
    let mut main_header = vec![0u8; 256];
    file.read_exact(&mut main_header)?;
    
    // 解析信号数量
    let signals_str = String::from_utf8_lossy(&main_header[252..256]);
    let total_signals: i32 = signals_str.trim().parse()?;
    println!("总信号数: {}", total_signals);
    
    // 读取信号头部
    let signal_header_size = total_signals as usize * 256;
    let mut signal_header = vec![0u8; signal_header_size];
    file.read_exact(&mut signal_header)?;
    
    // 检查每个信号的标签
    for i in 0..total_signals as usize {
        let label_start = i * 16;
        let label_bytes = &signal_header[label_start..label_start + 16];
        let label = String::from_utf8_lossy(label_bytes);
        
        println!("信号 {}: 标签='{}'", i, label.trim());
        println!("  原始字节: {:?}", label_bytes);
        println!("  是否为注释信号: {}", label_bytes == b"EDF Annotations ");
        
        // 检查样本数
        let samples_start = total_signals as usize * 216 + i * 8;
        let samples_str = String::from_utf8_lossy(&signal_header[samples_start..samples_start + 8]);
        let samples_per_record: i32 = samples_str.trim().parse().unwrap_or(0);
        println!("  每记录样本数: {}", samples_per_record);
        
        if label_bytes == b"EDF Annotations " {
            println!("  ✓ 找到注释信号！");
        }
    }
    
    // 现在使用EdfReader测试
    println!("\n=== 使用 EdfReader 测试 ===");
    let reader = EdfReader::open(filename)?;
    let header = reader.header();
    
    println!("EdfReader 报告:");
    println!("  用户可见信号数: {}", header.signals.len());
    println!("  头部中的注释数: {}", header.annotations_in_file);
    println!("  实际读取的注释数: {}", reader.annotations().len());
    
    // 如果没有读取到注释，让我们检查原始数据
    if reader.annotations().is_empty() {
        println!("\n=== 检查原始TAL数据 ===");
        check_raw_tal_data(filename, total_signals)?;
    }
    
    Ok(())
}

fn check_raw_tal_data(filename: &str, total_signals: i32) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open(filename)?;
    
    // 计算头部大小
    let header_size = (total_signals as usize + 1) * 256;
    println!("头部大小: {} 字节", header_size);
    
    // 跳到第一个数据记录
    file.seek(SeekFrom::Start(header_size as u64))?;
    
    // 计算数据记录大小
    // 假设：1个用户信号(10样本) + 1个注释信号(60样本) = 10*2 + 60*2 = 140字节
    let record_size = 140;
    let mut record_data = vec![0u8; record_size];
    file.read_exact(&mut record_data)?;
    
    println!("第一个数据记录 ({} 字节):", record_size);
    
    // 用户信号数据 (前20字节)
    println!("用户信号数据 (前20字节):");
    for (i, &byte) in record_data[0..20].iter().enumerate() {
        if i % 16 == 0 {
            print!("\n{:04x}: ", i);
        }
        print!("{:02x} ", byte);
    }
    println!();
    
    // 注释信号数据 (从字节20开始)
    println!("\n注释信号数据 (从字节20开始，前32字节):");
    for (i, &byte) in record_data[20..52].iter().enumerate() {
        if i % 16 == 0 {
            print!("\n{:04x}: ", i + 20);
        }
        print!("{:02x} ", byte);
    }
    println!();
    
    // 尝试解析为ASCII
    println!("\n注释数据 (ASCII):");
    for &byte in record_data[20..52].iter() {
        if byte >= 32 && byte <= 126 {
            print!("{}", byte as char);
        } else if byte == 0x14 {
            print!("\\x14");
        } else if byte == 0x15 {
            print!("\\x15");
        } else if byte == 0 {
            print!("\\0");
        } else {
            print!("\\x{:02x}", byte);
        }
    }
    println!();
    
    Ok(())
}
