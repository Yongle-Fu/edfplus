use edfplus::{EdfWriter, EdfReader, SignalParam, Result};

fn main() -> Result<()> {
    println!("=== EDF+ 注释功能测试 ===");
    
    // 创建包含注释的 EDF+ 文件
    let mut writer = EdfWriter::create("annotation_test.edf")?;
    writer.set_patient_info("P001", "M", "01-JAN-1990", "注释测试患者")?;
    
    // 添加一个信号
    let signal = SignalParam {
        label: "EEG Fp1".to_string(),
        samples_in_file: 0,
        physical_max: 200.0,
        physical_min: -200.0,
        digital_max: 32767,
        digital_min: -32768,
        samples_per_record: 256, // 256 Hz
        physical_dimension: "uV".to_string(),
        prefilter: "HP:0.1Hz LP:70Hz".to_string(),
        transducer: "AgAgCl electrodes".to_string(),
    };
    
    writer.add_signal(signal)?;
    
    // 添加各种类型的注释
    writer.add_annotation(0.0, None, "Recording start")?;
    writer.add_annotation(1.5, Some(2.0), "Sleep stage 1")?;
    writer.add_annotation(5.0, None, "Eye movement")?;
    writer.add_annotation(8.2, Some(0.5), "Spindle")?;
    writer.add_annotation(12.0, None, "Arousal")?;
    
    println!("添加了 {} 个注释", writer.annotation_count());
    
    // 写入10秒钟的模拟EEG数据
    for second in 0..10 {
        let mut samples = Vec::new();
        for i in 0..256 {
            let t = (second * 256 + i) as f64 / 256.0;
            // 模拟10Hz正弦波，带有一些噪声
            let signal_value = 50.0 * (2.0 * std::f64::consts::PI * 10.0 * t).sin() 
                             + 10.0 * (2.0 * std::f64::consts::PI * 40.0 * t).sin()
                             + 5.0 * (t * 123.456).sin(); // 噪声
            samples.push(signal_value);
        }
        writer.write_samples(&[samples])?;
    }
    
    writer.finalize()?;
    println!("EDF+ 文件创建完成: annotation_test.edf");
    
    // 读取并验证文件
    let mut reader = EdfReader::open("annotation_test.edf")?;
    let header = reader.header();
    
    println!("\n=== 文件验证 ===");
    println!("信号数量: {}", header.signals.len());
    println!("文件持续时间: {:.2} 秒", header.file_duration as f64 / 10_000_000.0);
    println!("患者信息: {} ({})", header.patient_name, header.patient_code);
    
    // 验证注释
    let annotations = reader.annotations();
    println!("\n=== 注释验证 ===");
    println!("读取到 {} 个注释:", annotations.len());
    
    for (i, annotation) in annotations.iter().enumerate() {
        let onset_seconds = annotation.onset as f64 / 10_000_000.0;
        let duration_seconds = if annotation.duration >= 0 {
            annotation.duration as f64 / 10_000_000.0
        } else {
            -1.0
        };
        
        if duration_seconds >= 0.0 {
            println!("  {}. {:.2}s (持续 {:.2}s): {}", 
                i + 1, onset_seconds, duration_seconds, annotation.description);
        } else {
            println!("  {}. {:.2}s (瞬时事件): {}", 
                i + 1, onset_seconds, annotation.description);
        }
    }
    
    // 验证信号数据
    println!("\n=== 信号数据验证 ===");
    let samples = reader.read_physical_samples(0, 1000)?;
    println!("读取前1000个样本: 范围 {:.2} 到 {:.2} uV", 
        samples.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
        samples.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)));
    
    // 清理测试文件
    // std::fs::remove_file("annotation_test.edf").ok();
    
    println!("\n✅ 注释功能测试完成！");
    Ok(())
}
