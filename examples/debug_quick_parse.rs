use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 调试 quick_parse_tal_for_count ===");
    
    // 手动读取TAL数据
    let mut file = File::open("debug_signal_detection.edf")?;
    
    // 跳到数据记录 (头部是768字节)
    file.seek(SeekFrom::Start(768))?;
    
    // 读取第一个数据记录 (140字节)
    let mut record_data = vec![0u8; 140];
    file.read_exact(&mut record_data)?;
    
    // 注释信号从字节20开始，长度120字节 (60样本 * 2字节)
    let tal_data = &record_data[20..140];
    
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
    
    // 测试计数逻辑
    println!("\n=== 测试 quick_parse_tal_for_count ===");
    
    // 测试1: 作为第一个记录的第一个注释信号
    let (count1, subsecond1) = quick_parse_tal_for_count_debug(tal_data, true, true)?;
    println!("作为第一个记录的第一个注释信号: count={}, subsecond={}", count1, subsecond1);
    
    // 测试2: 作为第一个记录的非第一个注释信号
    let (count2, subsecond2) = quick_parse_tal_for_count_debug(tal_data, true, false)?;
    println!("作为第一个记录的非第一个注释信号: count={}, subsecond={}", count2, subsecond2);
    
    // 测试3: 作为非第一个记录的第一个注释信号
    let (count3, subsecond3) = quick_parse_tal_for_count_debug(tal_data, false, true)?;
    println!("作为非第一个记录的第一个注释信号: count={}, subsecond={}", count3, subsecond3);
    
    Ok(())
}

fn quick_parse_tal_for_count_debug(data: &[u8], is_first_record: bool, is_first_annotation_signal: bool) -> Result<(i64, i64), Box<dyn std::error::Error>> {
    let mut count = 0i64;
    let mut subsecond = 0i64;
    let max = data.len();
    
    println!("\n--- quick_parse_tal_for_count 调试 ---");
    println!("数据长度: {}, 最后字节: 0x{:02x}", max, data[max-1]);
    println!("is_first_record: {}, is_first_annotation_signal: {}", is_first_record, is_first_annotation_signal);
    
    if max == 0 || data[max - 1] != 0 {
        println!("提前返回: 数据为空或不以null结尾");
        return Ok((0, 0));
    }
    
    let mut k = 0;
    let mut onset = false;
    let mut duration_start = false;
    let mut n = 0;
    let mut scratchpad = vec![0u8; max + 16];
    let mut zero = 0;
    let mut annots_in_tal = 0;
    let mut annots_in_record = 0;
    
    while k < max - 1 {
        let byte = data[k];
        
        if byte == 0 {
            if zero == 0 {
                if k > 0 && data[k - 1] != 20 {
                    println!("格式错误：null字节前应该是分隔符，位置{}", k);
                    break;
                }
                n = 0;
                onset = false;
                duration_start = false;
                scratchpad.fill(0);
                annots_in_tal = 0;
            }
            zero += 1;
            k += 1;
            continue;
        }
        
        if zero > 1 {
            println!("格式错误：连续的null字节太多，位置{}", k);
            break;
        }
        zero = 0;
        
        // 处理TAL分隔符
        if byte == 20 || byte == 21 { // 0x14 (20) 或 0x15 (21)
            if byte == 21 { // Duration分隔符
                if duration_start || onset || annots_in_tal > 0 {
                    println!("格式错误：duration分隔符位置错误，位置{}", k);
                    break;
                }
                duration_start = true;
                println!("开始duration字段，位置{}", k);
            }
            
            if byte == 20 && onset && !duration_start {
                // 描述字段结束 - 应用与parse_tal_data相同的逻辑
                let should_count = if is_first_annotation_signal && annots_in_record == 0 {
                    println!("第一个注释信号的第一个注释，可能是时间戳注释，位置{}", k);
                    false
                } else {
                    println!("应该计算的注释，位置{}", k);
                    true
                };
                
                if should_count {
                    count += 1;
                    println!("  -> 添加注释，当前计数: {}", count);
                }
                
                annots_in_tal += 1;
                annots_in_record += 1;
                n = 0;
                k += 1;
                continue;
            }
            
            if !onset {
                // Onset字段结束
                scratchpad[n] = 0;
                println!("时间戳结束，位置{}，内容: '{}'", k, String::from_utf8_lossy(&scratchpad[0..n]));
                onset = true;
                n = 0;
                k += 1;
                continue;
            }
            
            if duration_start {
                // Duration字段结束
                scratchpad[n] = 0;
                println!("持续时间结束，位置{}，内容: '{}'", k, String::from_utf8_lossy(&scratchpad[0..n]));
                duration_start = false;
                n = 0;
                k += 1;
                continue;
            }
        } else {
            // 常规字符
            if byte == b'+' && !onset && n == 0 {
                println!("跳过起始'+'号，位置{}", k);
                k += 1;
                continue;
            }
            
            if n < scratchpad.len() - 1 {
                scratchpad[n] = byte;
                n += 1;
            }
        }
        
        k += 1;
    }
    
    println!("最终计数: {}", count);
    Ok((count, subsecond))
}
