use edfplus::{EdfReader, EdfWriter, SignalParam};
use std::fs;
use std::path::Path;

// 清理测试文件的辅助函数
fn cleanup_test_file(filename: &str) {
    if Path::new(filename).exists() {
        fs::remove_file(filename).ok();
    }
}

// 创建测试信号的辅助函数
fn create_test_signal() -> SignalParam {
    SignalParam {
        label: "EEG Test".to_string(),
        samples_in_file: 0,
        physical_max: 100.0,
        physical_min: -100.0,
        digital_max: 32767,
        digital_min: -32768,
        samples_per_record: 256,
        physical_dimension: "uV".to_string(),
        prefilter: "HP:0.1Hz LP:70Hz".to_string(),
        transducer: "Test electrodes".to_string(),
    }
}

#[test]
fn test_basic_annotation_write_read() {
    let filename = "test_basic_annotations.edf";
    
    // 写入阶段 - 创建包含注释的文件
    {
        let mut writer = EdfWriter::create(filename).unwrap();
        writer.set_patient_info("ANN001", "F", "15-JUL-1985", "Annotation Test").unwrap();
        
        let signal = create_test_signal();
        writer.add_signal(signal).unwrap();
        
        // 添加各种类型的注释
        writer.add_annotation(0.0, None, "Recording Start").unwrap();
        writer.add_annotation(1.5, Some(2.0), "Sleep Stage N1").unwrap();
        writer.add_annotation(3.5, None, "Eye Movement").unwrap();
        writer.add_annotation(5.2, Some(0.5), "Artifact").unwrap();
        writer.add_annotation(7.8, None, "K-Complex").unwrap();
        
        // 写入10秒的数据
        for second in 0..10 {
            let mut samples = Vec::new();
            for i in 0..256 {
                let t = (second * 256 + i) as f64 / 256.0;
                let value = 30.0 * (2.0 * std::f64::consts::PI * 10.0 * t).sin();
                samples.push(value);
            }
            writer.write_samples(&[samples]).unwrap();
        }
        
        writer.finalize().unwrap();
    }
    
    // 读取阶段 - 验证注释
    {
        let reader = EdfReader::open(filename).unwrap();
        let annotations = reader.annotations();
        
        // 验证注释数量
        assert_eq!(annotations.len(), 5);
        
        // 验证具体注释内容
        let expected_annotations = vec![
            (0.0, None, "Recording Start"),
            (1.5, Some(2.0), "Sleep Stage N1"),
            (3.5, None, "Eye Movement"),
            (5.2, Some(0.5), "Artifact"),
            (7.8, None, "K-Complex"),
        ];
        
        for (i, (expected_onset, expected_duration, expected_desc)) in expected_annotations.iter().enumerate() {
            let annotation = &annotations[i];
            
            // 验证时间（转换回秒）
            let actual_onset = annotation.onset as f64 / 10_000_000.0;
            let tolerance = 0.001; // 1ms 容错
            assert!((actual_onset - expected_onset).abs() < tolerance,
                   "Annotation {} onset mismatch: expected {}, got {}", 
                   i, expected_onset, actual_onset);
            
            // 验证持续时间
            match expected_duration {
                Some(expected_dur) => {
                    assert!(annotation.duration >= 0);
                    let actual_duration = annotation.duration as f64 / 10_000_000.0;
                    assert!((actual_duration - expected_dur).abs() < tolerance,
                           "Annotation {} duration mismatch: expected {}, got {}", 
                           i, expected_dur, actual_duration);
                }
                None => {
                    assert_eq!(annotation.duration, -1, "Expected instantaneous event");
                }
            }
            
            // 验证描述
            assert_eq!(annotation.description, *expected_desc);
            
            println!("Annotation {}: {:.3}s - {} (duration: {:?})", 
                    i, actual_onset, annotation.description, 
                    if annotation.duration >= 0 { 
                        Some(annotation.duration as f64 / 10_000_000.0) 
                    } else { 
                        None 
                    });
        }
    }
    
    cleanup_test_file(filename);
}

#[test]
fn test_annotation_time_precision() {
    let filename = "test_precision_annotations.edf";
    
    // 写入阶段 - 测试高精度时间
    {
        let mut writer = EdfWriter::create(filename).unwrap();
        writer.set_patient_info("PREC001", "X", "X", "Precision Test").unwrap();
        
        let signal = create_test_signal();
        writer.add_signal(signal).unwrap();
        
        // 添加高精度时间的注释
        writer.add_annotation(0.0001, None, "Microsecond Event").unwrap();      // 0.1ms
        writer.add_annotation(0.1234567, None, "High Precision").unwrap();      // 123.4567ms
        writer.add_annotation(1.9999999, Some(0.0000001), "Nanosecond Duration").unwrap(); // 100ns duration
        writer.add_annotation(3.141592653589793, None, "Pi Seconds").unwrap();  // π秒
        
        // 写入5秒的数据
        for second in 0..5 {
            let mut samples = Vec::new();
            for i in 0..256 {
                let t = (second * 256 + i) as f64 / 256.0;
                let value = 20.0 * (2.0 * std::f64::consts::PI * 5.0 * t).sin();
                samples.push(value);
            }
            writer.write_samples(&[samples]).unwrap();
        }
        
        writer.finalize().unwrap();
    }
    
    // 读取阶段 - 验证精度
    {
        let reader = EdfReader::open(filename).unwrap();
        let annotations = reader.annotations();
        
        assert_eq!(annotations.len(), 4);
        
        // 验证高精度时间（EDF+内部使用100纳秒单位）
        let precision_tests = vec![
            (0.0001, "Microsecond Event"),
            (0.1234567, "High Precision"),
            (1.9999999, "Nanosecond Duration"),
            (3.141592653589793, "Pi Seconds"),
        ];
        
        for (i, (expected_time, expected_desc)) in precision_tests.iter().enumerate() {
            let annotation = &annotations[i];
            let actual_time = annotation.onset as f64 / 10_000_000.0;
            
            // 100纳秒精度测试
            let tolerance = 1e-7; // 100纳秒
            assert!((actual_time - expected_time).abs() < tolerance,
                   "High precision time test failed for '{}': expected {:.9}, got {:.9}",
                   expected_desc, expected_time, actual_time);
            
            assert_eq!(annotation.description, *expected_desc);
            
            println!("Precision test {}: Expected {:.9}s, Actual {:.9}s, Diff: {:.2e}s",
                    i, expected_time, actual_time, (actual_time - expected_time).abs());
        }
    }
    
    cleanup_test_file(filename);
}

#[test]
fn test_annotation_edge_cases() {
    let filename = "test_edge_annotations.edf";
    
    // 写入阶段 - 测试边界情况
    {
        let mut writer = EdfWriter::create(filename).unwrap();
        writer.set_patient_info("EDGE001", "X", "X", "Edge Case Test").unwrap();
        
        let signal = create_test_signal();
        writer.add_signal(signal).unwrap();
        
        // 测试各种边界情况的注释
        writer.add_annotation(0.0, None, "Exactly at start").unwrap();
        writer.add_annotation(0.0, Some(0.0), "Zero duration").unwrap();
        writer.add_annotation(59.999, None, "Near end").unwrap();
        
        // 测试长描述
        let long_description = "This is a very long annotation description that tests the system's ability to handle extended text content in annotations, which might be useful for detailed clinical observations and notes.";
        writer.add_annotation(30.0, Some(10.0), long_description).unwrap();
        
        // 测试特殊字符
        writer.add_annotation(45.0, None, "Special chars: àáâãäåæçèéêë 测试 🧠").unwrap();
        
        // 写入60秒的数据
        for second in 0..60 {
            let mut samples = Vec::new();
            for i in 0..256 {
                let t = (second * 256 + i) as f64 / 256.0;
                let value = 25.0 * (2.0 * std::f64::consts::PI * 8.0 * t).sin();
                samples.push(value);
            }
            writer.write_samples(&[samples]).unwrap();
        }
        
        writer.finalize().unwrap();
    }
    
    // 读取阶段 - 验证边界情况
    {
        let reader = EdfReader::open(filename).unwrap();
        let annotations = reader.annotations();
        
        assert_eq!(annotations.len(), 5);
        
        // 验证起始时间的注释
        let start_annotation = &annotations[0];
        assert_eq!(start_annotation.onset, 0);
        assert_eq!(start_annotation.description, "Exactly at start");
        
        // 验证零持续时间
        let zero_duration = &annotations[1];
        assert_eq!(zero_duration.onset, 0);
        assert_eq!(zero_duration.duration, 0);
        assert_eq!(zero_duration.description, "Zero duration");
        
        // 验证长描述
        let long_desc_annotation = annotations.iter()
            .find(|a| a.description.starts_with("This is a very long"))
            .expect("Should find long description annotation");
        assert!(long_desc_annotation.description.len() > 100);
        
        // 验证特殊字符
        let special_char_annotation = annotations.iter()
            .find(|a| a.description.contains("Special chars"))
            .expect("Should find special character annotation");
        assert!(special_char_annotation.description.contains("àáâãäåæçèéêë"));
        assert!(special_char_annotation.description.contains("测试"));
        assert!(special_char_annotation.description.contains("🧠"));
        
        println!("Edge case tests passed:");
        for (i, annotation) in annotations.iter().enumerate() {
            let onset_s = annotation.onset as f64 / 10_000_000.0;
            let duration_s = if annotation.duration >= 0 {
                Some(annotation.duration as f64 / 10_000_000.0)
            } else {
                None
            };
            println!("  {}: {:.3}s - {} (len: {}, duration: {:?})",
                    i, onset_s, &annotation.description[..annotation.description.len().min(50)],
                    annotation.description.len(), duration_s);
        }
    }
    
    cleanup_test_file(filename);
}

#[test]
fn test_multiple_annotation_channels() {
    let filename = "test_multi_annotation_channels.edf";
    
    // 写入阶段 - 测试多注释通道
    {
        let mut writer = EdfWriter::create(filename).unwrap();
        writer.set_patient_info("MULTI001", "X", "X", "Multi Annotation Test").unwrap();
        
        // 设置3个注释通道
        writer.set_number_of_annotation_signals(3).unwrap();
        
        let signal = create_test_signal();
        writer.add_signal(signal).unwrap();
        
        // 添加大量注释以测试多通道分发
        for i in 0..15 {
            let onset = i as f64 * 0.5; // 每0.5秒一个注释
            let description = format!("Event {}", i + 1);
            
            if i % 3 == 0 {
                // 长持续时间事件
                writer.add_annotation(onset, Some(2.0), &description).unwrap();
            } else {
                // 瞬时事件
                writer.add_annotation(onset, None, &description).unwrap();
            }
        }
        
        // 写入10秒的数据
        for second in 0..10 {
            let mut samples = Vec::new();
            for i in 0..256 {
                let t = (second * 256 + i) as f64 / 256.0;
                let value = 35.0 * (2.0 * std::f64::consts::PI * 12.0 * t).sin();
                samples.push(value);
            }
            writer.write_samples(&[samples]).unwrap();
        }
        
        writer.finalize().unwrap();
    }
    
    // 读取阶段 - 验证多通道注释
    {
        let reader = EdfReader::open(filename).unwrap();
        let annotations = reader.annotations();
        
        // 应该有15个注释
        assert_eq!(annotations.len(), 15);
        
        // 验证注释按时间排序
        for i in 1..annotations.len() {
            assert!(annotations[i].onset >= annotations[i-1].onset,
                   "Annotations should be sorted by onset time");
        }
        
        // 验证注释分布
        let mut event_counts = std::collections::HashMap::new();
        for annotation in annotations {
            let counter = event_counts.entry(&annotation.description).or_insert(0);
            *counter += 1;
        }
        
        // 每个事件应该只出现一次
        for (event, count) in &event_counts {
            assert_eq!(*count, 1, "Event '{}' should appear exactly once", event);
        }
        
        println!("Multi-channel annotation test:");
        println!("  Total annotations: {}", annotations.len());
        println!("  Unique events: {}", event_counts.len());
        
        for (i, annotation) in annotations.iter().enumerate() {
            let onset_s = annotation.onset as f64 / 10_000_000.0;
            let duration_s = if annotation.duration >= 0 {
                Some(annotation.duration as f64 / 10_000_000.0)
            } else {
                None
            };
            println!("    {}: {:.1}s - {} (duration: {:?})",
                    i, onset_s, annotation.description, duration_s);
        }
    }
    
    cleanup_test_file(filename);
}

#[test]
fn test_annotation_validation() {
    let filename = "test_validation_annotations.edf";
    
    // 测试注释验证
    {
        let mut writer = EdfWriter::create(filename).unwrap();
        writer.set_patient_info("VAL001", "X", "X", "Validation Test").unwrap();
        
        let signal = create_test_signal();
        writer.add_signal(signal).unwrap();
        
        // 测试有效的注释
        assert!(writer.add_annotation(1.0, None, "Valid annotation").is_ok());
        assert!(writer.add_annotation(2.0, Some(1.5), "Valid with duration").is_ok());
        
        // 测试无效的注释
        assert!(writer.add_annotation(-1.0, None, "Negative onset").is_err());
        assert!(writer.add_annotation(1.0, Some(-1.0), "Negative duration").is_err());
        assert!(writer.add_annotation(1.0, None, "").is_err()); // 空描述
        
        // 测试过长的描述
        let very_long_desc = "x".repeat(600);
        assert!(writer.add_annotation(1.0, None, &very_long_desc).is_err());
        
        // 写入基本数据
        let samples = vec![10.0; 256];
        writer.write_samples(&[samples]).unwrap();
        writer.finalize().unwrap();
    }
    
    // 验证只有有效的注释被保存
    {
        let reader = EdfReader::open(filename).unwrap();
        let annotations = reader.annotations();
        
        // 应该只有2个有效的注释
        assert_eq!(annotations.len(), 2);
        
        assert_eq!(annotations[0].description, "Valid annotation");
        assert_eq!(annotations[1].description, "Valid with duration");
        
        println!("Validation test passed: {} valid annotations saved", annotations.len());
    }
    
    cleanup_test_file(filename);
}

#[test]
fn test_sleep_study_annotations() {
    let filename = "test_sleep_study.edf";
    
    // 写入阶段 - 模拟完整的睡眠研究
    {
        let mut writer = EdfWriter::create(filename).unwrap();
        writer.set_patient_info("SLEEP001", "F", "22-AUG-1978", "Sleep Study Patient").unwrap();
        
        // 添加多个EEG通道
        for channel in &["C3-A2", "C4-A1", "O1-A2", "O2-A1"] {
            let mut signal = create_test_signal();
            signal.label = format!("EEG {}", channel);
            writer.add_signal(signal).unwrap();
        }
        
        // 添加睡眠研究典型的注释
        writer.add_annotation(0.0, None, "Lights Out").unwrap();
        writer.add_annotation(180.0, None, "Sleep Onset").unwrap();
        
        // 睡眠阶段
        writer.add_annotation(300.0, Some(1800.0), "Stage N1").unwrap();   // 5-35分钟
        writer.add_annotation(2100.0, Some(3600.0), "Stage N2").unwrap();  // 35-95分钟
        writer.add_annotation(5700.0, Some(1800.0), "Stage N3").unwrap();  // 95-125分钟
        writer.add_annotation(7500.0, Some(900.0), "REM Sleep").unwrap();  // 125-140分钟
        
        // 睡眠事件
        writer.add_annotation(1200.0, None, "Sleep Spindle").unwrap();
        writer.add_annotation(1800.0, None, "K-Complex").unwrap();
        writer.add_annotation(3600.0, None, "Vertex Sharp Wave").unwrap();
        writer.add_annotation(6000.0, None, "Delta Wave Burst").unwrap();
        writer.add_annotation(7800.0, None, "REM Burst").unwrap();
        writer.add_annotation(8100.0, None, "Eye Movement").unwrap();
        
        // 觉醒和artifacts
        writer.add_annotation(4200.0, Some(30.0), "Brief Awakening").unwrap();
        writer.add_annotation(6900.0, Some(15.0), "Movement Artifact").unwrap();
        writer.add_annotation(8400.0, None, "Final Awakening").unwrap();
        
        // 写入2.5小时的数据 (9000秒)
        for second in 0..9000 {
            let mut all_samples = Vec::new();
            
            for _channel in 0..4 {
                let mut channel_samples = Vec::new();
                for sample in 0..256 {
                    let t = (second * 256 + sample) as f64 / 256.0;
                    
                    // 根据时间模拟不同的脑电活动
                    let base_freq = match second {
                        0..=299 => 10.0,      // 觉醒时的alpha波
                        300..=2099 => 8.0,    // N1阶段
                        2100..=5699 => 5.0,   // N2阶段
                        5700..=7499 => 2.0,   // N3阶段（深睡）
                        7500..=8399 => 15.0,  // REM阶段
                        _ => 12.0,            // 觉醒
                    };
                    
                    let amplitude = match second {
                        5700..=7499 => 80.0,  // 深睡时高幅度
                        _ => 30.0,            // 其他阶段正常幅度
                    };
                    
                    let value = amplitude * (2.0 * std::f64::consts::PI * base_freq * t).sin() +
                               5.0 * (2.0 * std::f64::consts::PI * 50.0 * t).sin(); // 电力线干扰
                    
                    channel_samples.push(value);
                }
                all_samples.push(channel_samples);
            }
            
            writer.write_samples(&all_samples).unwrap();
        }
        
        writer.finalize().unwrap();
    }
    
    // 读取阶段 - 验证睡眠研究数据
    {
        let reader = EdfReader::open(filename).unwrap();
        let header = reader.header();
        let annotations = reader.annotations();
        
        // 验证文件结构
        assert_eq!(header.signals.len(), 4);
        assert_eq!(header.patient_name, "Sleep Study Patient");
        
        // 验证注释数量和类型
        assert_eq!(annotations.len(), 15);
        
        // 按类型分类注释
        let mut stage_annotations = Vec::new();
        let mut event_annotations = Vec::new();
        let mut other_annotations = Vec::new();
        
        for annotation in annotations {
            if annotation.description.starts_with("Stage") || annotation.description.contains("REM") {
                stage_annotations.push(annotation);
            } else if annotation.description.contains("Spindle") || 
                     annotation.description.contains("Complex") ||
                     annotation.description.contains("Wave") ||
                     annotation.description.contains("Burst") ||
                     annotation.description.contains("Eye Movement") {
                event_annotations.push(annotation);
            } else {
                other_annotations.push(annotation);
            }
        }
        
        println!("Sleep Study Analysis:");
        println!("  Total recording duration: {:.1} hours", 
                header.file_duration as f64 / 10_000_000.0 / 3600.0);
        println!("  Sleep stages: {}", stage_annotations.len());
        println!("  Sleep events: {}", event_annotations.len());
        println!("  Other annotations: {}", other_annotations.len());
        
        println!("\nSleep Stages:");
        for annotation in &stage_annotations {
            let onset_min = annotation.onset as f64 / 10_000_000.0 / 60.0;
            let duration_min = if annotation.duration > 0 {
                annotation.duration as f64 / 10_000_000.0 / 60.0
            } else {
                0.0
            };
            println!("    {:.1}-{:.1}min: {}", 
                    onset_min, onset_min + duration_min, annotation.description);
        }
        
        println!("\nSleep Events:");
        for annotation in &event_annotations {
            let onset_min = annotation.onset as f64 / 10_000_000.0 / 60.0;
            println!("    {:.1}min: {}", onset_min, annotation.description);
        }
    }
    
    cleanup_test_file(filename);
}
