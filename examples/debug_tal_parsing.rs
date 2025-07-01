use edfplus::{EdfWriter, EdfReader, SignalParam, Result};

fn main() -> Result<()> {
    println!("=== TAL解析调试工具 ===");
    
    // 创建一个简单的测试文件
    let mut writer = EdfWriter::create("debug_tal_parsing.edf")?;
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
    
    // 写入1秒的数据 (1个数据记录)
    let samples: Vec<f64> = (0..10).map(|j| j as f64).collect();
    writer.write_samples(&[samples])?;
    
    writer.finalize()?;
    println!("测试文件创建完成");
    
    // 手动检查TAL数据和解析过程
    check_tal_data_manually("debug_tal_parsing.edf")?;
    
    Ok(())
}

fn check_tal_data_manually(filename: &str) -> Result<()> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    
    println!("\n=== 手动TAL数据检查 ===");
    
    let mut file = File::open(filename)?;
    
    // 跳过头部 (2个信号，所以头部是3*256=768字节)
    file.seek(SeekFrom::Start(768))?;
    
    // 读取第一个数据记录
    let mut data_record = vec![0u8; 140]; // 10*2 + 120 = 140字节
    file.read_exact(&mut data_record)?;
    
    // 注释信号从字节20开始，长度120字节
    let tal_data = &data_record[20..140];
    
    println!("TAL数据 (120字节):");
    for (i, &byte) in tal_data.iter().enumerate() {
        if i % 16 == 0 {
            print!("\n{:04x}: ", i);
        }
        print!("{:02x} ", byte);
    }
    println!();
    
    println!("\nTAL数据 (ASCII显示):");
    for (i, &byte) in tal_data.iter().enumerate() {
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
    
    // 手动解析TAL数据
    parse_tal_manually(tal_data);
    
    Ok(())
}

fn parse_tal_manually(data: &[u8]) {
    println!("\n=== 手动解析TAL数据 ===");
    
    let mut k = 0;
    let max = data.len();
    let mut annotation_count = 0;
    
    // 遍历整个TAL数据
    while k < max {
        let byte = data[k];
        
        // 遇到null字节，跳过
        if byte == 0 {
            k += 1;
            continue;
        }
        
        if byte == b'+' {
            // 找到注释开始
            println!("在位置{}发现注释开始 '+'", k);
            k += 1; // 跳过'+'
            
            // 读取时间
            let time_start = k;
            while k < max && data[k] != 0x14 && data[k] != 0x15 && data[k] != 0 {
                k += 1;
            }
            
            if k > time_start {
                let time_str = String::from_utf8_lossy(&data[time_start..k]);
                println!("发现时间: '{}'", time_str);
                
                // 检查是否有持续时间
                if k < max && data[k] == 0x15 {
                    k += 1; // 跳过duration分隔符
                    let duration_start = k;
                    while k < max && data[k] != 0x14 && data[k] != 0 {
                        k += 1;
                    }
                    if k > duration_start {
                        let duration_str = String::from_utf8_lossy(&data[duration_start..k]);
                        println!("发现持续时间: '{}'", duration_str);
                    }
                }
                
                // 读取描述
                if k < max && data[k] == 0x14 {
                    k += 1; // 跳过description分隔符
                    let desc_start = k;
                    while k < max && data[k] != 0x14 && data[k] != 0 {
                        k += 1;
                    }
                    
                    if k > desc_start {
                        let description = String::from_utf8_lossy(&data[desc_start..k]);
                        println!("发现描述: '{}'", description);
                        
                        // 如果描述不为空，这是一个真正的注释
                        if !description.is_empty() {
                            annotation_count += 1;
                            println!("  -> 这是第{}个有效注释", annotation_count);
                        } else {
                            println!("  -> 这是时间戳注释（空描述）");
                        }
                    } else {
                        println!("发现空描述");
                        println!("  -> 这是时间戳注释（空描述）");
                    }
                    
                    if k < max && data[k] == 0x14 {
                        k += 1; // 跳过结束分隔符
                    }
                }
            }
        } else {
            k += 1;
        }
    }
    
    println!("手动解析结果: 找到 {} 个有效注释", annotation_count);
}
