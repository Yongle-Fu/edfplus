// 在 parse_tal_data 方法中加入调试输出
use edfplus::types::Annotation;

fn parse_tal_data_debug(data: &[u8], is_first_annotation_signal: bool) -> Result<Vec<Annotation>, Box<dyn std::error::Error>> {
    let mut annotations = Vec::new();
    let max = data.len();
    
    if max == 0 || data[max - 1] != 0 {
        return Ok(annotations);
    }
    
    let mut k = 0;
    let mut onset = false;
    let mut duration = false;
    let mut duration_start = false;
    let mut n = 0;
    let mut scratchpad = vec![0u8; max + 16];
    let mut time_in_txt = vec![0u8; 32];
    let mut duration_in_txt = vec![0u8; 32];
    let mut zero = 0;
    let mut annots_in_tal = 0;
    let mut annots_in_record = 0;
    
    println!("开始解析TAL数据，长度: {}", max);
    println!("前50字节: {:?}", &data[..50.min(max)]);
    
    while k < max - 1 {
        let byte = data[k];
        
        if byte == 0 {
            if zero == 0 {
                if k > 0 && data[k - 1] != 20 {
                    println!("格式错误：null字节前应该是分隔符，位置: {}", k);
                    break;
                }
                n = 0;
                onset = false;
                duration = false;
                duration_start = false;
                scratchpad.fill(0);
                annots_in_tal = 0;
            }
            zero += 1;
            k += 1;
            continue;
        }
        
        if zero > 1 {
            println!("格式错误：连续的null字节太多，位置: {}", k);
            break;
        }
        zero = 0;
        
        // 处理TAL分隔符
        if byte == 20 || byte == 21 { // 0x14 (20) 或 0x15 (21)
            if byte == 21 { // Duration分隔符
                if duration || duration_start || onset || annots_in_tal > 0 {
                    println!("格式错误：duration字段位置错误，位置: {}", k);
                    break;
                }
                duration_start = true;
                println!("找到duration分隔符，位置: {}", k);
            }
            
            if byte == 20 && onset && !duration_start {
                // 描述字段结束
                let description = if n > 0 {
                    String::from_utf8_lossy(&scratchpad[0..n]).to_string()
                } else {
                    String::new()
                };
                
                println!("处理注释 {}: 描述='{}'", annots_in_record, description);
                
                let is_timestamp_annotation = is_first_annotation_signal && 
                                               annots_in_record == 0 && 
                                               description.is_empty();
                
                println!("  是时间戳注释: {}", is_timestamp_annotation);
                
                if !is_timestamp_annotation {
                    let time_str = String::from_utf8_lossy(&time_in_txt)
                        .trim_end_matches('\0').to_string();
                    
                    println!("  创建注释: onset='{}', 描述='{}'", time_str, description);
                    
                    if let Ok(onset_seconds) = time_str.parse::<f64>() {
                        let onset_time = (onset_seconds * 10_000_000.0) as i64;
                        
                        let duration_time = if duration {
                            let duration_str = String::from_utf8_lossy(&duration_in_txt)
                                .trim_end_matches('\0').to_string();
                            if let Ok(duration_seconds) = duration_str.parse::<f64>() {
                                (duration_seconds * 10_000_000.0) as i64
                            } else {
                                -1
                            }
                        } else {
                            -1
                        };
                        
                        annotations.push(Annotation {
                            onset: onset_time,
                            duration: duration_time,
                            description,
                        });
                        
                        println!("  ✓ 注释已添加");
                    } else {
                        println!("  ✗ 无法解析onset时间: '{}'", time_str);
                    }
                } else {
                    println!("  - 跳过时间戳注释");
                }
                
                annots_in_tal += 1;
                annots_in_record += 1;
                
                // 重置状态变量
                onset = false;
                duration = false;
                duration_start = false;
                n = 0;
                scratchpad.fill(0);
                time_in_txt.fill(0);
                duration_in_txt.fill(0);
                k += 1;
                continue;
            }
            
            if !onset {
                // Onset字段结束
                scratchpad[n] = 0;
                
                let onset_str = String::from_utf8_lossy(&scratchpad[0..n]);
                println!("找到onset: '{}'，位置: {}", onset_str, k);
                
                // 复制onset时间
                let copy_len = n.min(time_in_txt.len() - 1);
                time_in_txt[..copy_len].copy_from_slice(&scratchpad[..copy_len]);
                time_in_txt[copy_len] = 0;
                
                onset = true;
                n = 0;
                k += 1;
                continue;
            }
            
            if duration_start {
                // Duration字段结束
                scratchpad[n] = 0;
                
                let duration_str = String::from_utf8_lossy(&scratchpad[0..n]);
                println!("找到duration: '{}'，位置: {}", duration_str, k);
                
                // 复制duration
                let copy_len = n.min(duration_in_txt.len() - 1);
                duration_in_txt[..copy_len].copy_from_slice(&scratchpad[..copy_len]);
                duration_in_txt[copy_len] = 0;
                
                duration = true;
                duration_start = false;
                n = 0;
                k += 1;
                continue;
            }
        } else {
            // 常规字符
            if byte == b'+' && !onset && n == 0 {
                println!("找到新注释开始 (+)，位置: {}", k);
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
    
    println!("TAL解析完成，找到 {} 个注释", annotations.len());
    Ok(annotations)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 从实际文件中提取TAL数据进行测试
    let tal_data = b"+0\x14\x14\0+0\x14Start\x14+0\x150\x14Zero duration\x14+0.5\x14Half second\x14\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
    
    println!("=== 调试TAL解析器 ===");
    let annotations = parse_tal_data_debug(tal_data, true)?;
    
    println!("\n最终结果: {} 个注释", annotations.len());
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
    
    Ok(())
}
