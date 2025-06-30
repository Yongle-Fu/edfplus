use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use chrono::{NaiveDate, NaiveTime, Datelike, Timelike};

use crate::types::{FileType, SignalParam};
use crate::error::{EdfError, Result};
use crate::EDFLIB_TIME_DIMENSION;

pub struct EdfWriter {
    file: BufWriter<File>,
    signals: Vec<SignalParam>,
    file_type: FileType,
    start_date: NaiveDate,
    start_time: NaiveTime,
    datarecord_duration: i64,
    samples_written: usize,
    header_written: bool,
    
    // EDF+ 字段
    patient_code: String,
    sex: String,
    birthdate: String,
    patient_name: String,
    patient_additional: String,
    admin_code: String,
    technician: String,
    equipment: String,
    recording_additional: String,
}

impl EdfWriter {
    /// 创建新的EDF+文件写入器
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::create(&path)
            .map_err(|e| EdfError::FileNotFound(format!("{}: {}", path.as_ref().display(), e)))?;
        
        let writer = BufWriter::new(file);
        
        // 使用默认日期时间
        let default_date = NaiveDate::from_ymd_opt(1985, 1, 1).unwrap();
        let default_time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        
        Ok(EdfWriter {
            file: writer,
            signals: Vec::new(),
            file_type: FileType::EdfPlus,
            start_date: default_date,
            start_time: default_time,
            datarecord_duration: EDFLIB_TIME_DIMENSION, // 1秒
            samples_written: 0,
            header_written: false,
            patient_code: "X".to_string(),
            sex: "X".to_string(),
            birthdate: "X".to_string(),
            patient_name: "X".to_string(),
            patient_additional: "X".to_string(),
            admin_code: "X".to_string(),
            technician: "X".to_string(),
            equipment: "X".to_string(),
            recording_additional: "X".to_string(),
        })
    }
    
    /// 添加一个信号
    pub fn add_signal(&mut self, signal: SignalParam) -> Result<()> {
        if self.header_written {
            return Err(EdfError::InvalidFormat("Cannot add signal after writing header".to_string()));
        }
        
        // 验证信号参数
        if signal.physical_min == signal.physical_max {
            return Err(EdfError::PhysicalMinEqualsMax);
        }
        if signal.digital_min == signal.digital_max {
            return Err(EdfError::DigitalMinEqualsMax);
        }
        
        self.signals.push(signal);
        Ok(())
    }
    
    /// 设置患者信息
    pub fn set_patient_info(&mut self, code: &str, sex: &str, birthdate: &str, name: &str) -> Result<()> {
        if self.header_written {
            return Err(EdfError::InvalidFormat("Cannot modify patient info after writing header".to_string()));
        }
        
        self.patient_code = code.to_string();
        self.sex = sex.to_string();
        self.birthdate = birthdate.to_string();
        self.patient_name = name.to_string();
        Ok(())
    }
    
    /// 写入头部
    fn write_header(&mut self, total_datarecords: i64) -> Result<()> {
        if self.header_written {
            return Ok(());
        }
        
        // 添加注释信号
        let annotation_signal = SignalParam {
            label: "EDF Annotations".to_string(),
            samples_in_file: total_datarecords,
            physical_max: 1.0,
            physical_min: -1.0,
            digital_max: 32767,
            digital_min: -32768,
            samples_per_record: 1,
            physical_dimension: "".to_string(),
            prefilter: "".to_string(),
            transducer: "".to_string(),
        };
        
        let total_signals = self.signals.len() + 1; // +1 for annotation
        let header_size = (total_signals + 1) * 256;
        
        // 写入主头部 (256字节)
        let mut main_header = vec![0u8; 256];
        
        // 版本 (8字节)
        main_header[0..8].copy_from_slice(b"0       ");
        
        // 患者信息字段 (80字节)
        let patient_field = format!("{} {} {} {} {}", 
            self.patient_code, self.sex, self.birthdate, self.patient_name, self.patient_additional);
        let patient_bytes = patient_field.as_bytes();
        let patient_len = patient_bytes.len().min(80);
        main_header[8..8+patient_len].copy_from_slice(&patient_bytes[..patient_len]);
        
        // 记录信息字段 (80字节)
        let recording_field = format!("Startdate {} {} {} {} {}", 
            self.start_date.format("%d-%b-%Y"), self.admin_code, self.technician, 
            self.equipment, self.recording_additional);
        let recording_bytes = recording_field.as_bytes();
        let recording_len = recording_bytes.len().min(80);
        main_header[88..88+recording_len].copy_from_slice(&recording_bytes[..recording_len]);
        
        // 开始日期 (8字节) "dd.mm.yy"
        let date_str = format!("{:02}.{:02}.{:02}", 
            self.start_date.day(), self.start_date.month(), self.start_date.year() % 100);
        main_header[168..176].copy_from_slice(date_str.as_bytes());
        
        // 开始时间 (8字节) "hh.mm.ss"
        let time_str = format!("{:02}.{:02}.{:02}", 
            self.start_time.hour(), self.start_time.minute(), self.start_time.second());
        main_header[176..184].copy_from_slice(time_str.as_bytes());
        
        // 头部大小 (8字节)
        let header_size_str = format!("{:<8}", header_size);
        main_header[184..192].copy_from_slice(header_size_str.as_bytes());
        
        // EDF+标识 (44字节)
        main_header[192..197].copy_from_slice(b"EDF+C");
        
        // 数据记录数 (8字节)
        let datarecords_str = format!("{:<8}", total_datarecords);
        main_header[236..244].copy_from_slice(datarecords_str.as_bytes());
        
        // 数据记录持续时间 (8字节)
        let duration_str = "1       ";
        main_header[244..252].copy_from_slice(duration_str.as_bytes());
        
        // 信号数 (4字节)
        let signals_str = format!("{:<4}", total_signals);
        main_header[252..256].copy_from_slice(signals_str.as_bytes());
        
        self.file.write_all(&main_header)?;
        
        // 写入信号头部
        let mut all_signals = self.signals.clone();
        all_signals.push(annotation_signal);
        
        // 每个字段写入所有信号
        for field_offset in 0..8 {
            for signal in &all_signals {
                let mut field_data = vec![0u8; 16];
                match field_offset {
                    0 => { // 标签
                        let label_bytes = signal.label.as_bytes();
                        let len = label_bytes.len().min(16);
                        field_data[..len].copy_from_slice(&label_bytes[..len]);
                    }
                    1 => { // 传感器
                        let mut field_data = vec![0u8; 80];
                        let trans_bytes = signal.transducer.as_bytes();
                        let len = trans_bytes.len().min(80);
                        field_data[..len].copy_from_slice(&trans_bytes[..len]);
                        self.file.write_all(&field_data)?;
                        continue;
                    }
                    2 => { // 物理单位
                        let mut field_data = vec![0u8; 8];
                        let unit_bytes = signal.physical_dimension.as_bytes();
                        let len = unit_bytes.len().min(8);
                        field_data[..len].copy_from_slice(&unit_bytes[..len]);
                        self.file.write_all(&field_data)?;
                        continue;
                    }
                    3 => { // 物理最小值
                        let mut field_data = vec![0u8; 8];
                        let phys_min_str = format!("{:<8}", signal.physical_min);
                        field_data.copy_from_slice(phys_min_str.as_bytes());
                        self.file.write_all(&field_data)?;
                        continue;
                    }
                    4 => { // 物理最大值
                        let mut field_data = vec![0u8; 8];
                        let phys_max_str = format!("{:<8}", signal.physical_max);
                        field_data.copy_from_slice(phys_max_str.as_bytes());
                        self.file.write_all(&field_data)?;
                        continue;
                    }
                    5 => { // 数字最小值
                        let mut field_data = vec![0u8; 8];
                        let dig_min_str = format!("{:<8}", signal.digital_min);
                        field_data.copy_from_slice(dig_min_str.as_bytes());
                        self.file.write_all(&field_data)?;
                        continue;
                    }
                    6 => { // 数字最大值
                        let mut field_data = vec![0u8; 8];
                        let dig_max_str = format!("{:<8}", signal.digital_max);
                        field_data.copy_from_slice(dig_max_str.as_bytes());
                        self.file.write_all(&field_data)?;
                        continue;
                    }
                    7 => { // 预滤波
                        let mut field_data = vec![0u8; 80];
                        let prefilter_bytes = signal.prefilter.as_bytes();
                        let len = prefilter_bytes.len().min(80);
                        field_data[..len].copy_from_slice(&prefilter_bytes[..len]);
                        self.file.write_all(&field_data)?;
                        continue;
                    }
                    _ => {}
                }
                if field_offset == 0 {
                    self.file.write_all(&field_data)?;
                }
            }
        }
        
        // 每个数据记录的样本数
        for signal in &all_signals {
            let mut field_data = vec![0u8; 8];
            let samples_str = format!("{:<8}", signal.samples_per_record);
            field_data.copy_from_slice(samples_str.as_bytes());
            self.file.write_all(&field_data)?;
        }
        
        // 保留字段
        for _signal in &all_signals {
            let field_data = vec![0u8; 32];
            self.file.write_all(&field_data)?;
        }
        
        self.header_written = true;
        Ok(())
    }
    
    /// 写入物理样本数据
    pub fn write_samples(&mut self, samples: &[Vec<f64>]) -> Result<()> {
        if samples.len() != self.signals.len() {
            return Err(EdfError::InvalidFormat("Sample count must match signal count".to_string()));
        }
        
        // 确保所有信号的样本数相同
        let record_length = samples[0].len();
        for signal_samples in samples {
            if signal_samples.len() != record_length {
                return Err(EdfError::InvalidFormat("All signals must have same number of samples".to_string()));
            }
        }
        
        // 如果还没写头部，先写头部
        if !self.header_written {
            self.write_header(record_length as i64)?;
        }
        
        // 写入数据记录
        for record_idx in 0..record_length {
            // 写入每个信号的样本
            for (signal_idx, signal_samples) in samples.iter().enumerate() {
                let signal = &self.signals[signal_idx];
                let physical_value = signal_samples[record_idx];
                let digital_value = signal.to_digital(physical_value);
                
                // 转换为16位小端序
                let bytes = (digital_value as i16).to_le_bytes();
                self.file.write_all(&bytes)?;
            }
            
            // 写入注释信号的占位符数据
            let annotation_bytes = [0u8; 2];
            self.file.write_all(&annotation_bytes)?;
        }
        
        self.samples_written += record_length;
        Ok(())
    }
    
    /// 完成写入并关闭文件
    pub fn finalize(mut self) -> Result<()> {
        self.file.flush()?;
        Ok(())
    }
}
