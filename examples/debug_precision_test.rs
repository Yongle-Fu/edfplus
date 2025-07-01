use edfplus::{EdfWriter, EdfReader, SignalParam, Result};

fn main() -> Result<()> {
    println!("=== 调试精度测试注释 ===");
    
    let filename = "debug_precision.edf";
    
    // 重现精度测试的写入逻辑
    {
        let mut writer = EdfWriter::create(filename)?;
        writer.set_patient_info("PREC001", "X", "X", "Precision Test")?;
        
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
        
        // 添加与测试相同的注释
        writer.add_annotation(0.0001, None, "Microsecond Event")?;      // 0.1ms
        writer.add_annotation(0.1234567, None, "High Precision")?;      // 123.4567ms
        writer.add_annotation(1.9999999, Some(0.0000001), "Nanosecond Duration")?; // 100ns duration
        writer.add_annotation(3.141592653589793, None, "Pi Seconds")?;  // π秒
        
        println!("添加了 {} 个注释", writer.annotation_count());
        
        // 写入5秒的数据
        for second in 0..5 {
            let mut samples = Vec::new();
            for i in 0..256 {
                let t = (second * 256 + i) as f64 / 256.0;
                let value = 20.0 * (2.0 * std::f64::consts::PI * 5.0 * t).sin();
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
            println!("  {}: {:.7}s - '{}' (持续时间: {:?})", 
                    i, onset_s, annotation.description, duration_s);
        }
    }
    
    std::fs::remove_file(filename).ok();
    Ok(())
}
