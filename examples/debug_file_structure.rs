use edfplus::{EdfWriter, SignalParam};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

fn create_test_signal() -> SignalParam {
    SignalParam {
        label: "EEG Channel".to_string(),
        samples_in_file: 0,
        physical_max: 100.0,
        physical_min: -100.0,
        digital_max: 32767,
        digital_min: -32768,
        samples_per_record: 256,
        physical_dimension: "uV".to_string(),
        prefilter: "HP:0.1Hz LP:70Hz".to_string(),
        transducer: "AgAgCl electrodes".to_string(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filename = "debug_file_structure.edf";
    
    // 创建一个简化的测试文件
    {
        let mut writer = EdfWriter::create(filename)?;
        writer.set_patient_info("TEST001", "X", "X", "Structure Test")?;
        
        let signal = create_test_signal();
        writer.add_signal(signal)?;
        
        // 添加一个简单的注释用于测试
        writer.add_annotation(0.5, None, "Test")?;
        
        // 写入1秒的数据
        let mut samples = Vec::new();
        for i in 0..256 {
            samples.push(i as f64);
        }
        writer.write_samples(&[samples])?;
        
        writer.finalize()?;
    }
    
    // 分析文件结构
    println!("=== EDF文件结构分析 ===");
    {
        let mut file = File::open(filename)?;
        
        // 读取主头部
        let mut header = vec![0u8; 256];
        file.read_exact(&mut header)?;
        
        let signals_str = String::from_utf8_lossy(&header[252..256]);
        let total_signals: i32 = signals_str.trim().parse()?;
        
        let datarecords_str = String::from_utf8_lossy(&header[236..244]);
        let datarecords: i32 = datarecords_str.trim().parse()?;
        
        println!("总信号数: {}", total_signals);
        println!("数据记录数: {}", datarecords);
        
        // 读取信号头部分析
        let signal_header_size = total_signals as usize * 256;
        let mut signal_header = vec![0u8; signal_header_size];
        file.read_exact(&mut signal_header)?;
        
        println!("\n=== 信号头部分析 ===");
        for i in 0..total_signals as usize {
            // 标签
            let label_start = i * 16;
            let label_bytes = &signal_header[label_start..label_start + 16];
            let label_str = String::from_utf8_lossy(label_bytes);
            let label = label_str.trim();
            
            // 每记录样本数
            let samples_start = total_signals as usize * 216 + i * 8;
            let samples_str = String::from_utf8_lossy(&signal_header[samples_start..samples_start + 8]);
            let samples_per_record: i32 = samples_str.trim().parse().unwrap_or(0);
            
            let is_annotation = label_bytes == b"EDF Annotations ";
            
            println!("信号 {}: '{}' - {} 样本/记录 (注释信号: {})", 
                    i, label, samples_per_record, is_annotation);
        }
        
        // 计算数据记录结构
        println!("\n=== 数据记录结构 ===");
        let mut buffer_offset = 0;
        for i in 0..total_signals as usize {
            let samples_start = total_signals as usize * 216 + i * 8;
            let samples_str = String::from_utf8_lossy(&signal_header[samples_start..samples_start + 8]);
            let samples_per_record: i32 = samples_str.trim().parse().unwrap_or(0);
            
            let bytes_per_signal = samples_per_record as usize * 2;
            
            let label_start = i * 16;
            let label_bytes = &signal_header[label_start..label_start + 16];
            let label_str = String::from_utf8_lossy(label_bytes);
            let label = label_str.trim();
            
            println!("信号 {}: 偏移 {} - {} 字节 ({})", 
                    i, buffer_offset, bytes_per_signal, label);
            
            buffer_offset += bytes_per_signal;
        }
        
        let total_record_size = buffer_offset;
        println!("总数据记录大小: {} 字节", total_record_size);
        
        // 读取第一个数据记录并分析
        println!("\n=== 第一个数据记录分析 ===");
        let header_size = 256 + signal_header_size;
        file.seek(SeekFrom::Start(header_size as u64))?;
        
        let mut record_data = vec![0u8; total_record_size];
        file.read_exact(&mut record_data)?;
        
        // 分析每个信号的数据
        buffer_offset = 0;
        for i in 0..total_signals as usize {
            let samples_start = total_signals as usize * 216 + i * 8;
            let samples_str = String::from_utf8_lossy(&signal_header[samples_start..samples_start + 8]);
            let samples_per_record: i32 = samples_str.trim().parse().unwrap_or(0);
            
            let bytes_per_signal = samples_per_record as usize * 2;
            
            let label_start = i * 16;
            let label_bytes = &signal_header[label_start..label_start + 16];
            let label_str = String::from_utf8_lossy(label_bytes);
            let label = label_str.trim();
            let is_annotation = label_bytes == b"EDF Annotations ";
            
            println!("\n--- 信号 {}: {} ---", i, label);
            
            if is_annotation {
                // 分析注释数据
                let signal_data = &record_data[buffer_offset..buffer_offset + bytes_per_signal];
                println!("注释数据 (前60字节):");
                print!("十六进制: ");
                for j in 0..60.min(signal_data.len()) {
                    print!("{:02x} ", signal_data[j]);
                }
                println!();
                
                print!("ASCII: ");
                for j in 0..60.min(signal_data.len()) {
                    let byte = signal_data[j];
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
            } else {
                // 分析常规信号数据
                let signal_data = &record_data[buffer_offset..buffer_offset + bytes_per_signal];
                println!("常规信号数据 (前10个样本):");
                for j in 0..10.min(samples_per_record as usize) {
                    let sample_offset = j * 2;
                    if sample_offset + 1 < signal_data.len() {
                        let sample = i16::from_le_bytes([
                            signal_data[sample_offset],
                            signal_data[sample_offset + 1]
                        ]);
                        print!("{} ", sample);
                    }
                }
                println!();
            }
            
            buffer_offset += bytes_per_signal;
        }
    }
    
    std::fs::remove_file(filename).ok();
    Ok(())
}
