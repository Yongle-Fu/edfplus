use edfplus::{EdfWriter, EdfReader, SignalParam, Result};

fn main() -> Result<()> {
    println!("=== 调试边界情况注释测试 ===");
    
    let filename = "debug_edge_cases.edf";
    
    // 重现边界情况测试的写入逻辑
    {
        let mut writer = EdfWriter::create(filename)?;
        writer.set_patient_info("EDGE001", "X", "X", "Edge Case Test")?;
        
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
        
        // 添加边界情况的注释
        println!("添加注释:");
        
        let result1 = writer.add_annotation(0.0, None, "Exactly at start");
        println!("  1. Exactly at start (0.0s): {:?}", result1);
        
        let result2 = writer.add_annotation(0.0, Some(0.0), "Zero duration");
        println!("  2. Zero duration (0.0s, 0.0s): {:?}", result2);
        
        let result3 = writer.add_annotation(59.999, None, "Near end");
        println!("  3. Near end (59.999s): {:?}", result3);
        
        // 测试长描述
        let long_description = "This is a very long annotation description that tests the system's ability to handle extended text content in annotations, which might be useful for detailed clinical observations and notes.";
        let result4 = writer.add_annotation(30.0, Some(10.0), long_description);
        println!("  4. Long description (30.0s, 10.0s): {:?}", result4);
        
        // 测试特殊字符
        let result5 = writer.add_annotation(45.0, None, "Special chars: àáâãäåæçèéêë 测试 🧠");
        println!("  5. Special chars (45.0s): {:?}", result5);
        
        println!("\n添加了 {} 个注释", writer.annotation_count());
        
        // 写入60秒的数据
        for second in 0..60 {
            let mut samples = Vec::new();
            for i in 0..256 {
                let t = (second * 256 + i) as f64 / 256.0;
                let value = 25.0 * (2.0 * std::f64::consts::PI * 8.0 * t).sin();
                samples.push(value);
            }
            writer.write_samples(&[samples])?;
        }
        
        writer.finalize()?;
    }
    
    // 读取并分析
    {
        let reader = EdfReader::open(filename)?;
        let annotations = reader.annotations();
        
        println!("\n读取到 {} 个注释:", annotations.len());
        for (i, annotation) in annotations.iter().enumerate() {
            let onset_s = annotation.onset as f64 / 10_000_000.0;
            let duration_s = if annotation.duration >= 0 {
                Some(annotation.duration as f64 / 10_000_000.0)
            } else {
                None
            };
            println!("  {}: {:.3}s - '{}' (持续时间: {:?})", 
                    i, onset_s, 
                    &annotation.description[..annotation.description.len().min(50)],
                    duration_s);
        }
    }
    
    std::fs::remove_file(filename).ok();
    Ok(())
}
