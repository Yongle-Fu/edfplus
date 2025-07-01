use edfplus::{EdfWriter, EdfReader, SignalParam, Result};
use std::io::{Read, Seek, SeekFrom};

fn main() -> Result<()> {
    println!("=== 注释调试工具 ===");
    
    // 创建一个简单的测试文件
    let mut writer = EdfWriter::create("debug_annotations.edf")?;
    writer.set_patient_info("P001", "M", "01-JAN-1990", "Debug Patient")?;
    
    let signal = SignalParam {
        label: "Test Signal".to_string(),
        samples_in_file: 0,
        physical_max: 100.0,
        physical_min: -100.0,
        digital_max: 32767,
        digital_min: -32768,
        samples_per_record: 10, // 简化：每记录10个样本
        physical_dimension: "uV".to_string(),
        prefilter: "None".to_string(),
        transducer: "Test".to_string(),
    };
    
    writer.add_signal(signal)?;
    
    // 添加注释
    writer.add_annotation(0.5, None, "Test Event")?;
    println!("添加了注释: 0.5s, 'Test Event'");
    
    // 写入2秒的数据 (2个数据记录)
    for i in 0..2 {
        let samples: Vec<f64> = (0..10).map(|j| (i * 10 + j) as f64).collect();
        writer.write_samples(&[samples])?;
    }
    
    writer.finalize()?;
    println!("测试文件创建完成");
    
    // 现在手动检查文件内容
    let mut file = std::fs::File::open("debug_annotations.edf")?;
    
    // 读取头部信息
    let mut header = vec![0u8; 256];
    file.read_exact(&mut header)?;
    
    // 解析信号数量
    let signals_str = String::from_utf8_lossy(&header[252..256]);
    let total_signals: i32 = signals_str.trim().parse().unwrap_or(0);
    println!("总信号数: {}", total_signals);
    
    // 读取信号头部
    let signal_header_size = total_signals as usize * 256;
    let mut signal_header = vec![0u8; signal_header_size];
    file.read_exact(&mut signal_header)?;
    
    // 查看信号标签
    for i in 0..total_signals as usize {
        let label_start = i * 16;
        let label = String::from_utf8_lossy(&signal_header[label_start..label_start + 16]);
        println!("信号 {}: '{}'", i, label.trim());
        
        // 查看每记录样本数
        let samples_start = total_signals as usize * 216 + i * 8;
        let samples_str = String::from_utf8_lossy(&signal_header[samples_start..samples_start + 8]);
        let samples_per_record: i32 = samples_str.trim().parse().unwrap_or(0);
        println!("  每记录样本数: {}", samples_per_record);
    }
    
    // 现在读取数据记录
    println!("\n=== 数据记录检查 ===");
    
    // 跳到数据开始位置
    let data_start = 256 + signal_header_size;
    file.seek(SeekFrom::Start(data_start as u64))?;
    
    // 读取第一个数据记录
    let record_size = 10 * 2 + 120; // 数据信号 + 注释信号
    let mut first_record = vec![0u8; record_size];
    file.read_exact(&mut first_record)?;
    
    println!("第一个数据记录大小: {} 字节", record_size);
    
    // 显示数据信号部分 (前20字节 = 10样本 * 2字节)
    println!("数据信号部分:");
    for i in 0..10 {
        let sample_bytes = &first_record[i*2..(i+1)*2];
        let sample = i16::from_le_bytes([sample_bytes[0], sample_bytes[1]]);
        print!("{} ", sample);
    }
    println!();
    
    // 显示注释信号部分 (120字节)
    println!("\n注释信号部分:");
    let annotation_data = &first_record[20..140];
    
    // 以十六进制显示前50字节
    print!("Hex: ");
    for i in 0..50.min(annotation_data.len()) {
        print!("{:02x} ", annotation_data[i]);
    }
    println!();
    
    // 尝试解析为文本
    print!("ASCII: ");
    for i in 0..50.min(annotation_data.len()) {
        let byte = annotation_data[i];
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
    
    // 使用我们的读取器测试
    println!("\n=== 使用 EdfReader 测试 ===");
    let reader = EdfReader::open("debug_annotations.edf")?;
    let annotations = reader.annotations();
    println!("读取到 {} 个注释", annotations.len());
    
    for (i, annotation) in annotations.iter().enumerate() {
        println!("注释 {}: {:.2}s - '{}'", i, annotation.onset as f64 / 10_000_000.0, annotation.description);
    }
    
    // 保留文件用于检查
    println!("\n文件已保存为 debug_annotations.edf 用于进一步检查");
    
    Ok(())
}
