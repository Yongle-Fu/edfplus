use edfplus::{EdfWriter, SignalParam, Result};

fn main() -> Result<()> {
    println!("=== 测试 add_annotation 验证逻辑 ===");
    
    let mut writer = EdfWriter::create("validation_debug.edf")?;
    writer.set_patient_info("VAL001", "X", "X", "Validation Test")?;
    
    let signal = SignalParam {
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
    };
    
    writer.add_signal(signal)?;
    
    // 测试有效的注释
    println!("测试有效注释:");
    match writer.add_annotation(1.0, None, "Valid annotation") {
        Ok(()) => println!("  ✓ 'Valid annotation' 添加成功"),
        Err(e) => println!("  ✗ 'Valid annotation' 添加失败: {}", e),
    }
    
    match writer.add_annotation(2.0, Some(1.5), "Valid with duration") {
        Ok(()) => println!("  ✓ 'Valid with duration' 添加成功"),
        Err(e) => println!("  ✗ 'Valid with duration' 添加失败: {}", e),
    }
    
    println!("当前注释数量: {}", writer.annotation_count());
    
    // 测试无效的注释
    println!("\n测试无效注释:");
    match writer.add_annotation(-1.0, None, "Negative onset") {
        Ok(()) => println!("  ✗ 'Negative onset' 应该失败但成功了"),
        Err(e) => println!("  ✓ 'Negative onset' 正确被拒绝: {}", e),
    }
    
    match writer.add_annotation(1.0, Some(-1.0), "Negative duration") {
        Ok(()) => println!("  ✗ 'Negative duration' 应该失败但成功了"),
        Err(e) => println!("  ✓ 'Negative duration' 正确被拒绝: {}", e),
    }
    
    match writer.add_annotation(1.0, None, "") {
        Ok(()) => println!("  ✗ 空描述应该失败但成功了"),
        Err(e) => println!("  ✓ 空描述正确被拒绝: {}", e),
    }
    
    // 写入数据并完成文件
    let samples = vec![10.0; 256];
    writer.write_samples(&[samples])?;
    writer.finalize()?;
    
    std::fs::remove_file("validation_debug.edf").ok();
    Ok(())
}
