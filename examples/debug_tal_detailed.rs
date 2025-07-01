use edfplus::{EdfWriter, EdfReader, SignalParam};
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
    let filename = "debug_tal_detailed.edf";
    
    // 创建一个简化的测试，只添加有问题的注释
    {
        let mut writer = EdfWriter::create(filename)?;
        writer.set_patient_info("EDGE001", "X", "X", "Edge Case Test")?;
        
        let signal = create_test_signal();
        writer.add_signal(signal)?;
        
        // 只添加有问题的注释
        println!("添加问题注释:");
        
        // 这个应该被读取
        writer.add_annotation(0.0, None, "Exactly at start")?;
        println!("  ✓ 添加了: Exactly at start");
        
        // 这个丢失了 - 零持续时间
        writer.add_annotation(0.0, Some(0.0), "Zero duration")?;
        println!("  ✓ 添加了: Zero duration with 0.0s duration");
        
        // 这个丢失了 - 长描述有持续时间
        writer.add_annotation(30.0, Some(10.0), "Long description test")?;
        println!("  ✓ 添加了: Long description with 10.0s duration");
        
        println!("Writer中的注释数量: {}", writer.annotation_count());
        
        // 写入31秒的数据（足够包含30秒的注释）
        for second in 0..31 {
            let mut samples = Vec::new();
            for i in 0..256 {
                let t = (second * 256 + i) as f64 / 256.0;
                let value = 25.0 * (2.0 * std::f64::consts::PI * 8.0 * t).sin();
                samples.push(value);
            }
            writer.write_samples(&[samples])?;
        }
        
        writer.finalize()?;
    }
    
    // 查看原始TAL数据
    println!("\n=== 检查原始TAL数据 ===");
    {
        let mut file = File::open(filename)?;
        
        // 读取头部获取信息
        let mut header = vec![0u8; 256];
        file.read_exact(&mut header)?;
        
        let signals_str = String::from_utf8_lossy(&header[252..256]);
        let total_signals: i32 = signals_str.trim().parse()?;
        
        let datarecords_str = String::from_utf8_lossy(&header[236..244]);
        let datarecords: i32 = datarecords_str.trim().parse()?;
        
        println!("总信号数: {}, 数据记录数: {}", total_signals, datarecords);
        
        // 跳过信号头部，定位到第一个数据记录
        let header_size = (total_signals as usize + 1) * 256;
        file.seek(SeekFrom::Start(header_size as u64))?;
        
        // 检查前3个数据记录的注释信号数据
        for record in 0..3.min(datarecords) {
            println!("\n--- 数据记录 {} ---", record);
            
            // 跳过到正确的数据记录
            let record_size = 256 * 2 + 120; // EEG信号: 256样本×2字节 + 注释信号: 120字节
            let record_offset = header_size as u64 + (record as u64 * record_size as u64);
            file.seek(SeekFrom::Start(record_offset))?;
            
            // 跳过EEG信号数据（256样本 × 2字节 = 512字节）
            file.seek(SeekFrom::Current(512))?;
            
            // 读取注释信号数据 (120字节)
            let mut annotation_data = vec![0u8; 120];
            file.read_exact(&mut annotation_data)?;
            
            println!("注释数据 (前80字节):");
            for i in 0..80 {
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
        }
    }
    
    // 使用reader读取
    println!("\n=== 使用EdfReader读取 ===");
    {
        let reader = EdfReader::open(filename)?;
        let annotations = reader.annotations();
        
        println!("读取到 {} 个注释:", annotations.len());
        for (i, annotation) in annotations.iter().enumerate() {
            let onset_s = annotation.onset as f64 / 10_000_000.0;
            let duration_s = if annotation.duration >= 0 {
                Some(annotation.duration as f64 / 10_000_000.0)
            } else {
                None
            };
            println!("  {}: {:.6}s - '{}' (持续时间: {:?})", 
                    i, onset_s, annotation.description, duration_s);
        }
    }
    
    std::fs::remove_file(filename).ok();
    Ok(())
}
