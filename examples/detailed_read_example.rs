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
            
            // 克隆头部信息以避免借用冲突
            let header = reader.header().clone();
            
            // 显示文件基本信息
            println!("\n📊 文件信息:");
            println!("  文件类型: {:?}", header.file_type);
            println!("  信号数量: {}", header.signals.len());
            println!("  文件时长: {:.2} 秒", header.file_duration as f64 / 10_000_000.0);
            println!("  数据记录数: {}", header.datarecords_in_file);
            println!("  记录时长: {:.3} 秒", header.datarecord_duration as f64 / 10_000_000.0);
            
            // 显示患者信息
            println!("\n👤 患者信息:");
            println!("  患者代码: {}", header.patient_code);
            println!("  性别: {}", header.sex);
            println!("  出生日期: {}", header.birthdate);
            println!("  患者姓名: {}", header.patient_name);
            
            // 显示记录信息
            println!("\n🏥 记录信息:");
            println!("  开始日期: {}", header.start_date);
            println!("  开始时间: {}", header.start_time);
            println!("  设备: {}", header.equipment);
            println!("  技术员: {}", header.technician);
            
            // 显示每个信号的详细信息
            println!("\n📈 信号信息:");
            for (i, signal) in header.signals.iter().enumerate() {
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
            for (signal_idx, signal) in header.signals.iter().enumerate() {
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
            if !header.signals.is_empty() {
                let signal_idx = 0;
                let signal = &header.signals[signal_idx];
                
                // 定位到中间位置
                let mid_position = signal.samples_in_file / 2;
                reader.seek(signal_idx, mid_position)?;
                let current_pos = reader.tell(signal_idx)?;
                println!("  定位到信号 {} 的位置 {} (目标: {})", signal_idx, current_pos, mid_position);
                
                // 读取几个样本
                let samples = reader.read_physical_samples(signal_idx, 5)?;
                println!("  从中间位置读取的5个样本: {:?}", samples);
                
                // 回到开头
                reader.rewind(signal_idx)?;
                let pos_after_rewind = reader.tell(signal_idx)?;
                println!("  重置后位置: {}", pos_after_rewind);
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
