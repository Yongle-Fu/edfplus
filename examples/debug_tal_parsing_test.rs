use edfplus::{EdfWriter, SignalParam};

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
    let filename = "debug_tal_parsing.edf";
    
    // 创建一个包含问题注释的测试文件
    {
        let mut writer = EdfWriter::create(filename)?;
        writer.set_patient_info("TEST001", "X", "X", "TAL Parsing Test")?;
        
        let signal = create_test_signal();
        writer.add_signal(signal)?;
        
        // 添加测试注释
        println!("添加注释到writer:");
        
        // 第1个注释：在0.0秒
        writer.add_annotation(0.0, None, "Start")?;
        println!("  1. 0.0s - 'Start'");
        
        // 第2个注释：在0.0秒，但有持续时间
        writer.add_annotation(0.0, Some(0.0), "Zero duration")?;
        println!("  2. 0.0s - 'Zero duration' (持续时间: 0.0s)");
        
        // 第3个注释：在0.5秒
        writer.add_annotation(0.5, None, "Half second")?;
        println!("  3. 0.5s - 'Half second'");
        
        println!("Writer中的注释数量: {}", writer.annotation_count());
        
        // 写入2秒的数据
        for second in 0..2 {
            let mut samples = Vec::new();
            for i in 0..256 {
                let t = (second * 256 + i) as f64 / 256.0;
                let value = 10.0 * (2.0 * std::f64::consts::PI * 5.0 * t).sin();
                samples.push(value);
            }
            writer.write_samples(&[samples])?;
        }
        
        writer.finalize()?;
    }
    
    // 现在用我们的reader读取并检查TAL解析
    println!("\n=== 用EdfReader读取 ===");
    {
        use edfplus::EdfReader;
        let reader = EdfReader::open(filename)?;
        let annotations = reader.annotations();
        
        println!("Reader读取到 {} 个注释:", annotations.len());
        for (i, annotation) in annotations.iter().enumerate() {
            let onset_s = annotation.onset as f64 / 10_000_000.0;
            let duration_s = if annotation.duration >= 0 {
                Some(annotation.duration as f64 / 10_000_000.0)
            } else {
                None
            };
            println!("  {}: {:.3}s - '{}' (持续时间: {:?})", 
                    i, onset_s, annotation.description, duration_s);
        }
        
        // 验证预期
        if annotations.len() == 3 {
            println!("✓ 所有注释都被正确读取");
        } else {
            println!("✗ 注释数量不匹配！期望3个，实际{}个", annotations.len());
            
            // 额外调试：检查原始TAL数据
            println!("\n=== 原始TAL数据检查 ===");
            
            // 手动读取第一个数据记录的TAL数据
            use std::fs::File;
            use std::io::{Read, Seek, SeekFrom};
            
            let mut file = File::open(filename)?;
            
            // 跳过头部（256 + 2*256 = 768字节）
            file.seek(SeekFrom::Start(768))?;
            
            // 跳过EEG数据（256样本 * 2字节 = 512字节）
            file.seek(SeekFrom::Current(512))?;
            
            // 读取注释数据（60样本 * 2字节 = 120字节）
            let mut tal_data = vec![0u8; 120];
            file.read_exact(&mut tal_data)?;
            
            println!("第一个数据记录的TAL数据:");
            print!("ASCII: ");
            for &byte in &tal_data[..80] {
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
    
    std::fs::remove_file(filename).ok();
    Ok(())
}
