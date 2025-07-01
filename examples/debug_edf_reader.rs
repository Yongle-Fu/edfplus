use edfplus::{EdfWriter, EdfReader, SignalParam, Result};

fn main() -> Result<()> {
    println!("=== EdfReader 注释解析调试 ===");
    
    // 创建一个简单的测试文件
    let mut writer = EdfWriter::create("debug_reader.edf")?;
    writer.set_patient_info("P001", "M", "01-JAN-1990", "Debug Patient")?;
    
    let signal = SignalParam {
        label: "Test Signal".to_string(),
        samples_in_file: 0,
        physical_max: 100.0,
        physical_min: -100.0,
        digital_max: 32767,
        digital_min: -32768,
        samples_per_record: 10,
        physical_dimension: "uV".to_string(),
        prefilter: "None".to_string(),
        transducer: "Test".to_string(),
    };
    
    writer.add_signal(signal)?;
    
    // 添加注释
    writer.add_annotation(0.5, None, "Test Event")?;
    println!("写入了1个注释: 0.5s, 'Test Event'");
    
    // 写入1秒的数据
    let samples: Vec<f64> = (0..10).map(|j| j as f64).collect();
    writer.write_samples(&[samples])?;
    
    writer.finalize()?;
    println!("文件创建完成");
    
    // 现在使用EdfReader读取
    println!("\n=== 使用 EdfReader 读取 ===");
    let reader = EdfReader::open("debug_reader.edf")?;
    let annotations = reader.annotations();
    
    println!("EdfReader 读取到 {} 个注释", annotations.len());
    
    for (i, annotation) in annotations.iter().enumerate() {
        println!("注释 {}: {:.3}s - '{}'", 
            i, 
            annotation.onset as f64 / 10_000_000.0, 
            annotation.description
        );
    }
    
    // 检查头部信息
    let header = reader.header();
    println!("\n文件信息:");
    println!("  数据记录数: {}", header.datarecords_in_file);
    println!("  头部中的注释数: {}", header.annotations_in_file);
    println!("  信号数: {}", header.signals.len());
    
    Ok(())
}
