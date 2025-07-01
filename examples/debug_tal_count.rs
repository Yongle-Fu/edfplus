use std::fs::File;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 调试 quick_parse_tal_for_count ===");
    
    // 直接读取TAL数据
    let mut file = File::open("debug_reader.edf")?;
    
    // 跳过头部 (2个信号，所以头部是3*256=768字节)
    file.seek(SeekFrom::Start(768))?;
    
    // 读取第一个数据记录
    let mut data_record = vec![0u8; 140]; // 10*2 + 120 = 140字节
    file.read_exact(&mut data_record)?;
    
    // 注释信号从字节20开始，长度120字节
    let tal_data = &data_record[20..140];
    
    println!("TAL数据 (前32字节):");
    for (i, &byte) in tal_data.iter().take(32).enumerate() {
        if i % 16 == 0 {
            print!("\n{:04x}: ", i);
        }
        print!("{:02x} ", byte);
    }
    println!();
    
    println!("\nTAL数据 (ASCII显示):");
    for &byte in tal_data.iter().take(32) {
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
    
    // 手动测试计数逻辑
    let (count, subsecond) = quick_parse_tal_for_count_debug(tal_data, true)?;
    println!("\nquick_parse_tal_for_count 结果: count={}, subsecond={}", count, subsecond);
    
    Ok(())
}

fn quick_parse_tal_for_count_debug(data: &[u8], is_first_record: bool) -> Result<(i64, i64), Box<dyn std::error::Error>> {
    let mut count = 0i64;
    let mut subsecond = 0i64;
    let max = data.len();
    
    println!("\n=== quick_parse_tal_for_count 调试 ===");
    println!("数据长度: {}, 最后字节: 0x{:02x}", max, data[max-1]);
    
    if max == 0 || data[max - 1] != 0 {
        println!("提前返回: 数据为空或不以null结尾");
        return Ok((0, 0));
    }
    
    let mut k = 0;
    let mut in_description = false;
    let mut description_start = 0;
    let mut is_first_annotation = is_first_record;
    
    while k < max - 1 {
        let byte = data[k];
        
        if byte == 0 {
            println!("在位置{}遇到null字节，停止解析", k);
            break;
        }
        
        if byte == 20 { // 0x14 - TAL separator
            if in_description {
                // 描述结束 - 检查描述是否为空
                let desc_len = k - description_start;
                let is_empty_description = desc_len == 0;
                
                println!("描述结束：位置{}, 长度{}, 空描述:{}, 第一个注释:{}", 
                    k, desc_len, is_empty_description, is_first_annotation);
                
                // 如果是第一个注释且描述为空，这是时间戳注释，跳过
                if !(is_first_annotation && is_empty_description) {
                    count += 1;
                    println!("  -> 添加注释，当前计数: {}", count);
                } else {
                    println!("  -> 跳过时间戳注释");
                }
                
                is_first_annotation = false; // 只有第一个注释可能是时间戳
                in_description = false;
            } else {
                // 时间戳结束，开始描述
                println!("时间戳结束，开始描述，位置:{}", k);
                in_description = true;
                description_start = k + 1;
            }
        }
        
        k += 1;
    }
    
    println!("最终计数: {}", count);
    Ok((count, subsecond))
}
