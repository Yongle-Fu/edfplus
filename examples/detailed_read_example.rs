use edfplus::{EdfReader, Result};

fn main() -> Result<()> {
    println!("EDF+ 数据读取示例");
    println!("库版本: {}", edfplus::version());
    println!();
    
    // 尝试读取我们生成的测试文件
    let file_path = "test_data/test_generated.edf";
    
    match EdfReader::open(file_path) {
        Ok(mut reader) => {
            println!("✅ 成功打开文件: {}", file_path);
            
            // 获取头部信息的拷贝用于显示
            let header_info = {
                let header = reader.header();
                (
                    header.signals.len(),
                    header.file_duration,
                    header.datarecords_in_file,
                    header.datarecord_duration,
                    header.patient_code.clone(),
                    header.sex.clone(),
                    header.birthdate.clone(),
                    header.patient_name.clone(),
                    header.start_date,
                    header.start_time,
                    header.equipment.clone(),
                    header.technician.clone(),
                    header.signals.clone(), // 克隆整个信号列表
                )
            };
            
            let (signals_len, file_duration, datarecords_in_file, datarecord_duration,
                 patient_code, sex, birthdate, patient_name, start_date, start_time,
                 equipment, technician, signals) = header_info;
            
            // 显示文件基本信息
            println!("\n📊 文件信息:");
            println!("  文件格式: EDF+ (European Data Format Plus)");
            println!("  信号数量: {}", signals_len);
            println!("  文件时长: {:.2} 秒", file_duration as f64 / 10_000_000.0);
            println!("  数据记录数: {}", datarecords_in_file);
            println!("  记录时长: {:.3} 秒", datarecord_duration as f64 / 10_000_000.0);
            
            // 显示患者信息
            println!("\n👤 患者信息:");
            println!("  患者代码: {}", patient_code);
            println!("  性别: {}", sex);
            println!("  出生日期: {}", birthdate);
            println!("  患者姓名: {}", patient_name);
            
            // 显示记录信息
            println!("\n🏥 记录信息:");
            println!("  开始日期: {}", start_date);
            println!("  开始时间: {}", start_time);
            println!("  设备: {}", equipment);
            println!("  技术员: {}", technician);
            
            // 显示每个信号的详细信息
            println!("\n📈 信号信息:");
            for (i, signal) in signals.iter().enumerate() {
                println!("  信号 {}: {}", i, signal.label);
                println!("    物理范围: {:.2} - {:.2} {}", 
                    signal.physical_min, signal.physical_max, signal.physical_dimension);
                println!("    数字范围: {} - {}", 
                    signal.digital_min, signal.digital_max);
                println!("    每记录样本数: {}", signal.samples_per_record);
                println!("    总样本数: {}", signal.samples_in_file);
                println!("    预滤波: {}", signal.prefilter);
                println!("    传感器: {}", signal.transducer);
                println!();
            }
            
            // 读取并显示前10个样本的数据
            println!("📊 样本数据预览 (前10个样本):");
            for signal_idx in 0..signals.len() {
                let signal = &signals[signal_idx];
                
                // 注意：EDF文件中的注释信号不能用常规方法读取样本数据
                // EDF规范中字符串字段可能包含null字节，所以使用contains()检查更可靠
                if signal.label.contains("Annotation") {
                    println!("\n  {} (注释信号，跳过数据读取)", signal.label);
                    continue;
                }
                
                println!("\n  {} ({}):", signal.label, signal.physical_dimension);
                
                // 重置到文件开头
                reader.rewind(signal_idx)?;
                
                // 读取前10个物理样本
                let samples = reader.read_physical_samples(signal_idx, 10)?;
                print!("    ");
                for (i, sample) in samples.iter().enumerate() {
                    print!("{:8.2}", sample);
                    if i < samples.len() - 1 {
                        print!(", ");
                    }
                }
                println!();
                
                // 读取对应的数字样本用于验证
                reader.rewind(signal_idx)?;
                let digital_samples = reader.read_digital_samples(signal_idx, 10)?;
                print!("    数字值: ");
                for (i, sample) in digital_samples.iter().enumerate() {
                    print!("{:6}", sample);
                    if i < digital_samples.len() - 1 {
                        print!(", ");
                    }
                }
                println!();
            }
            
            // 测试定位功能
            println!("\n🎯 测试文件定位功能:");
            if !signals.is_empty() {
                // 找到第一个非注释信号
                let signal_idx = signals.iter().position(|s| !s.label.contains("Annotation")).unwrap_or(0);
                
                if !signals[signal_idx].label.contains("Annotation") {
                    // 确保重置到文件开头
                    reader.rewind(signal_idx)?;
                    let initial_pos = reader.tell(signal_idx)?;
                    println!("调试：重置后的初始位置: {}", initial_pos);
                    
                    // 从开头读取几个样本作为基准
                    let baseline_samples = reader.read_physical_samples(signal_idx, 3)?;
                    println!("  开头3个样本: {:?}", baseline_samples);
                    
                    // 重置后定位到位置 100
                    reader.rewind(signal_idx)?;
                    let test_position = 100;
                    reader.seek(signal_idx, test_position)?;
                    let current_pos = reader.tell(signal_idx)?;
                    println!("  定位到位置 {} (实际: {})", test_position, current_pos);
                    
                    // 读取定位后的样本
                    let positioned_samples = reader.read_physical_samples(signal_idx, 3)?;
                    println!("  位置 {} 的3个样本: {:?}", test_position, positioned_samples);
                    
                    // 验证样本确实不同
                    if baseline_samples != positioned_samples {
                        println!("  ✅ 定位功能正常工作 - 样本已改变");
                    } else {
                        println!("  ⚠️  定位可能有问题 - 样本相同");
                    }
                    
                    // 最后重置
                    reader.rewind(signal_idx)?;
                    println!("  重置完成");
                }
            }
            
            println!("\n✅ 测试完成！");
            
        }
        Err(e) => {
            println!("❌ 无法打开文件 {}: {}", file_path, e);
            println!("\n💡 提示: 请先运行以下命令生成测试文件:");
            println!("   cargo run --example generate_test_file");
        }
    }
    
    Ok(())
}
