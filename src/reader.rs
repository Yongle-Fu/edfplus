use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use chrono::{NaiveDate, NaiveTime};

use crate::types::{EdfHeader, SignalParam, FileType, Annotation};
use crate::error::{EdfError, Result};
use crate::utils::{atoi_nonlocalized, atof_nonlocalized, parse_edf_time};
use crate::EDFLIB_TIME_DIMENSION;

pub struct EdfReader {
    file: BufReader<File>,
    header: EdfHeader,
    /// 每个信号在文件中的位置信息
    signal_info: Vec<SignalInfo>,
    /// 当前每个信号的样本位置指针
    sample_positions: Vec<i64>,
    /// 文件的头部大小
    header_size: usize,
    /// 每个数据记录的大小（字节）
    record_size: usize,
    /// 注释列表
    annotations: Vec<Annotation>,
}

#[derive(Debug, Clone)]
struct SignalInfo {
    /// 信号在数据记录中的字节偏移
    buffer_offset: usize,
    /// 每个数据记录中的样本数
    samples_per_record: i32,
    /// 是否是注释信号
    is_annotation: bool,
}

impl EdfReader {
    /// 打开EDF+文件进行读取
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(&path)
            .map_err(|e| EdfError::FileNotFound(format!("{}: {}", path.as_ref().display(), e)))?;
        
        let mut reader = BufReader::new(file);
        
        // 读取并解析头部
        let (header, signal_info, record_size) = Self::parse_header(&mut reader)?;
        
        // 初始化样本位置指针
        let sample_positions = vec![0i64; header.signals.len()];
        
        // 读取注释（如果需要）
        let annotations = Vec::new(); // TODO: 实现注释读取
        
        Ok(EdfReader {
            file: reader,
            header,
            signal_info,
            sample_positions,
            header_size: 256, // 临时值，将在parse_header中正确设置
            record_size,
            annotations,
        })
    }
    
    /// 获取文件头信息
    pub fn header(&self) -> &EdfHeader {
        &self.header
    }
    
    /// 获取注释列表
    pub fn annotations(&self) -> &[Annotation] {
        &self.annotations
    }
    
    /// 读取指定信号的物理值样本
    pub fn read_physical_samples(&mut self, signal: usize, count: usize) -> Result<Vec<f64>> {
        let digital_samples = self.read_digital_samples(signal, count)?;
        
        if signal >= self.header.signals.len() {
            return Err(EdfError::InvalidSignalIndex(signal));
        }
        
        let signal_param = &self.header.signals[signal];
        let physical_samples = digital_samples
            .into_iter()
            .map(|d| signal_param.to_physical(d))
            .collect();
        
        Ok(physical_samples)
    }
    
    /// 读取指定信号的数字值样本
    pub fn read_digital_samples(&mut self, signal: usize, count: usize) -> Result<Vec<i32>> {
        if signal >= self.header.signals.len() {
            return Err(EdfError::InvalidSignalIndex(signal));
        }
        
        if count == 0 {
            return Ok(Vec::new());
        }
        
        let signal_info = &self.signal_info[signal];
        let signal_param = &self.header.signals[signal];
        
        // 计算可读取的最大样本数
        let samples_in_file = signal_param.samples_per_record as i64 * self.header.datarecords_in_file;
        let available_samples = (samples_in_file - self.sample_positions[signal]).max(0) as usize;
        let actual_count = count.min(available_samples);
        
        if actual_count == 0 {
            return Ok(Vec::new());
        }
        
        let mut samples = Vec::with_capacity(actual_count);
        let mut samples_read = 0;
        
        while samples_read < actual_count {
            let current_pos = self.sample_positions[signal];
            let record_index = current_pos / signal_param.samples_per_record as i64;
            let sample_in_record = current_pos % signal_param.samples_per_record as i64;
            
            // 计算文件偏移量
            let file_offset = self.header_size as u64 
                + record_index as u64 * self.record_size as u64
                + signal_info.buffer_offset as u64
                + sample_in_record as u64 * 2; // EDF每个样本2字节
            
            // 定位到正确位置
            self.file.seek(SeekFrom::Start(file_offset))?;
            
            // 计算在当前记录中可以读取的样本数
            let samples_in_current_record = (signal_param.samples_per_record as i64 - sample_in_record) as usize;
            let samples_to_read = (actual_count - samples_read).min(samples_in_current_record);
            
            // 读取样本
            for _ in 0..samples_to_read {
                let mut buf = [0u8; 2];
                self.file.read_exact(&mut buf)?;
                
                // 转换为有符号16位整数（小端序）
                let digital_value = i16::from_le_bytes(buf) as i32;
                
                // 应用数字范围限制
                let clamped_value = digital_value
                    .max(signal_param.digital_min)
                    .min(signal_param.digital_max);
                
                samples.push(clamped_value);
                samples_read += 1;
                
                if samples_read >= actual_count {
                    break;
                }
            }
            
            // 更新样本位置
            self.sample_positions[signal] = current_pos + samples_to_read as i64;
        }
        
        Ok(samples)
    }
    
    /// 设置指定信号的样本位置
    pub fn seek(&mut self, signal: usize, position: i64) -> Result<i64> {
        if signal >= self.header.signals.len() {
            return Err(EdfError::InvalidSignalIndex(signal));
        }
        
        let signal_param = &self.header.signals[signal];
        let max_position = signal_param.samples_per_record as i64 * self.header.datarecords_in_file;
        
        let new_position = position.max(0).min(max_position);
        self.sample_positions[signal] = new_position;
        
        Ok(new_position)
    }
    
    /// 获取指定信号的当前样本位置
    pub fn tell(&self, signal: usize) -> Result<i64> {
        if signal >= self.header.signals.len() {
            return Err(EdfError::InvalidSignalIndex(signal));
        }
        
        Ok(self.sample_positions[signal])
    }
    
    /// 将指定信号的位置重置到开头
    pub fn rewind(&mut self, signal: usize) -> Result<()> {
        self.seek(signal, 0)?;
        Ok(())
    }
    
    /// 解析EDF+文件头部
    fn parse_header(reader: &mut BufReader<File>) -> Result<(EdfHeader, Vec<SignalInfo>, usize)> {
        // 读取主头部（256字节）
        reader.seek(SeekFrom::Start(0))?;
        let mut main_header = vec![0u8; 256];
        reader.read_exact(&mut main_header)?;
        
        // 验证EDF+标识
        let version = String::from_utf8_lossy(&main_header[0..8]);
        if !version.trim().starts_with('0') {
            return Err(EdfError::UnsupportedFileType(format!("Not an EDF file: {}", version)));
        }
        
        // 解析信号数量
        let signals_str = String::from_utf8_lossy(&main_header[252..256]);
        let total_signal_count = atoi_nonlocalized(&signals_str);
        if total_signal_count < 1 || total_signal_count > crate::EDFLIB_MAXSIGNALS as i32 {
            return Err(EdfError::InvalidSignalCount(total_signal_count));
        }
        
        // 验证头部大小
        let header_size_str = String::from_utf8_lossy(&main_header[184..192]);
        let expected_header_size = (total_signal_count + 1) * 256;
        let actual_header_size = atoi_nonlocalized(&header_size_str);
        if actual_header_size != expected_header_size {
            return Err(EdfError::InvalidHeader);
        }
        
        // 检查EDF+标识
        let reserved = String::from_utf8_lossy(&main_header[192..236]);
        let is_edfplus = reserved.starts_with("EDF+C");
        if !is_edfplus {
            return Err(EdfError::UnsupportedFileType("Only EDF+ files are supported".to_string()));
        }
        
        // 解析基本信息
        let patient_field = String::from_utf8_lossy(&main_header[8..88]).trim().to_string();
        let recording_field = String::from_utf8_lossy(&main_header[88..168]).trim().to_string();
        
        // 解析日期和时间
        let date_str = String::from_utf8_lossy(&main_header[168..176]);
        let time_str = String::from_utf8_lossy(&main_header[176..184]);
        
        let (start_date, start_time) = Self::parse_datetime(&date_str, &time_str)?;
        
        // 解析数据记录信息
        let datarecords_str = String::from_utf8_lossy(&main_header[236..244]);
        let datarecords = atoi_nonlocalized(&datarecords_str) as i64;
        
        let duration_str = String::from_utf8_lossy(&main_header[244..252]);
        let datarecord_duration = if duration_str.trim() == "1" {
            EDFLIB_TIME_DIMENSION
        } else {
            parse_edf_time(&duration_str)?
        };
        
        // 读取信号头部信息
        let signal_header_size = total_signal_count as usize * 256;
        let mut signal_header = vec![0u8; signal_header_size];
        reader.read_exact(&mut signal_header)?;
        
        // 解析信号参数
        let (signals, signal_info, total_record_size) = Self::parse_signals(
            &signal_header, 
            total_signal_count as usize,
            datarecords
        )?;
        
        // 解析EDF+字段
        let (patient_code, sex, birthdate, patient_name, patient_additional) = 
            Self::parse_edfplus_patient(&patient_field)?;
        
        let (admin_code, technician, equipment, recording_additional) = 
            Self::parse_edfplus_recording(&recording_field)?;
        
        let header = EdfHeader {
            file_type: FileType::EdfPlus,
            signals,
            file_duration: datarecord_duration * datarecords,
            start_date,
            start_time,
            starttime_subsecond: 0, // TODO: 从注释中解析
            datarecords_in_file: datarecords,
            datarecord_duration,
            annotations_in_file: 0, // TODO: 计算注释数量
            patient_code,
            sex,
            birthdate,
            patient_name,
            patient_additional,
            admin_code,
            technician,
            equipment,
            recording_additional,
        };
        
        Ok((header, signal_info, total_record_size))
    }
    
    /// 解析日期时间
    fn parse_datetime(date_str: &str, time_str: &str) -> Result<(NaiveDate, NaiveTime)> {
        // 解析日期 "dd.mm.yy"
        let date_parts: Vec<&str> = date_str.split('.').collect();
        if date_parts.len() != 3 {
            return Err(EdfError::FormatError);
        }
        
        let day = atoi_nonlocalized(date_parts[0]);
        let month = atoi_nonlocalized(date_parts[1]);
        let year = {
            let yy = atoi_nonlocalized(date_parts[2]);
            if yy > 84 { 1900 + yy } else { 2000 + yy }
        };
        
        let start_date = NaiveDate::from_ymd_opt(year, month as u32, day as u32)
            .ok_or(EdfError::FormatError)?;
        
        // 解析时间 "hh.mm.ss"
        let time_parts: Vec<&str> = time_str.split('.').collect();
        if time_parts.len() != 3 {
            return Err(EdfError::FormatError);
        }
        
        let hour = atoi_nonlocalized(time_parts[0]);
        let minute = atoi_nonlocalized(time_parts[1]);
        let second = atoi_nonlocalized(time_parts[2]);
        
        let start_time = NaiveTime::from_hms_opt(hour as u32, minute as u32, second as u32)
            .ok_or(EdfError::FormatError)?;
        
        Ok((start_date, start_time))
    }
    
    /// 解析信号参数
    fn parse_signals(
        signal_header: &[u8], 
        total_signal_count: usize,
        datarecords: i64
    ) -> Result<(Vec<SignalParam>, Vec<SignalInfo>, usize)> {
        let mut signals = Vec::new();
        let mut signal_info = Vec::new();
        let mut buffer_offset = 0;
        
        // 解析每个信号的各个字段
        for i in 0..total_signal_count {
            // 标签 (16字节)
            let label_start = i * 16;
            let label = String::from_utf8_lossy(&signal_header[label_start..label_start + 16])
                .trim().to_string();
            
            // 检查是否是注释信号
            let is_annotation = label == "EDF Annotations";
            
            // 传感器类型 (80字节，从偏移16*signal_count开始)
            let transducer_start = total_signal_count * 16 + i * 80;
            let transducer = String::from_utf8_lossy(
                &signal_header[transducer_start..transducer_start + 80]
            ).trim().to_string();
            
            // 物理单位 (8字节)
            let unit_start = total_signal_count * 96 + i * 8;
            let physical_dimension = String::from_utf8_lossy(
                &signal_header[unit_start..unit_start + 8]
            ).trim().to_string();
            
            // 物理最小值 (8字节)
            let phys_min_start = total_signal_count * 104 + i * 8;
            let phys_min_str = String::from_utf8_lossy(
                &signal_header[phys_min_start..phys_min_start + 8]
            );
            let physical_min = atof_nonlocalized(&phys_min_str);
            
            // 物理最大值 (8字节)
            let phys_max_start = total_signal_count * 112 + i * 8;
            let phys_max_str = String::from_utf8_lossy(
                &signal_header[phys_max_start..phys_max_start + 8]
            );
            let physical_max = atof_nonlocalized(&phys_max_str);
            
            // 数字最小值 (8字节)
            let dig_min_start = total_signal_count * 120 + i * 8;
            let dig_min_str = String::from_utf8_lossy(
                &signal_header[dig_min_start..dig_min_start + 8]
            );
            let digital_min = atoi_nonlocalized(&dig_min_str);
            
            // 数字最大值 (8字节)  
            let dig_max_start = total_signal_count * 128 + i * 8;
            let dig_max_str = String::from_utf8_lossy(
                &signal_header[dig_max_start..dig_max_start + 8]
            );
            let digital_max = atoi_nonlocalized(&dig_max_str);
            
            // 预滤波 (80字节)
            let prefilter_start = total_signal_count * 136 + i * 80;
            let prefilter = String::from_utf8_lossy(
                &signal_header[prefilter_start..prefilter_start + 80]
            ).trim().to_string();
            
            // 每个数据记录中的样本数 (8字节)
            let samples_start = total_signal_count * 216 + i * 8;
            let samples_str = String::from_utf8_lossy(
                &signal_header[samples_start..samples_start + 8]
            );
            let samples_per_record = atoi_nonlocalized(&samples_str);
            
            let info = SignalInfo {
                buffer_offset,
                samples_per_record,
                is_annotation,
            };
            
            // 只有非注释信号才添加到用户可见的信号列表中
            if !is_annotation {
                // 验证参数
                if physical_min == physical_max {
                    return Err(EdfError::PhysicalMinEqualsMax);
                }
                if digital_min == digital_max {
                    return Err(EdfError::DigitalMinEqualsMax);
                }
                
                let signal_param = SignalParam {
                    label,
                    samples_in_file: samples_per_record as i64 * datarecords,
                    physical_max,
                    physical_min,
                    digital_max,
                    digital_min,
                    samples_per_record,
                    physical_dimension,
                    prefilter,
                    transducer,
                };
                
                signals.push(signal_param);
            }
            
            signal_info.push(info);
            
            // 更新缓冲区偏移（每个样本2字节）
            buffer_offset += samples_per_record as usize * 2;
        }
        
        Ok((signals, signal_info, buffer_offset))
    }
    
    /// 解析EDF+患者字段
    fn parse_edfplus_patient(patient_field: &str) -> Result<(String, String, String, String, String)> {
        // EDF+ 患者字段格式: "patientcode sex birthdate patientname additional_info"
        let parts: Vec<&str> = patient_field.split_whitespace().collect();
        
        let patient_code = parts.get(0).unwrap_or(&"").to_string();
        let sex = parts.get(1).unwrap_or(&"").to_string();
        let birthdate = parts.get(2).unwrap_or(&"").to_string();
        let patient_name = parts.get(3).unwrap_or(&"").to_string();
        let patient_additional = parts.get(4..).map(|s| s.join(" ")).unwrap_or_default();
        
        Ok((patient_code, sex, birthdate, patient_name, patient_additional))
    }
    
    /// 解析EDF+记录字段
    fn parse_edfplus_recording(recording_field: &str) -> Result<(String, String, String, String)> {
        // EDF+ 记录字段格式: "startdate admincode technician equipment additional_info"
        let parts: Vec<&str> = recording_field.split_whitespace().collect();
        
        let admin_code = parts.get(1).unwrap_or(&"").to_string();
        let technician = parts.get(2).unwrap_or(&"").to_string();
        let equipment = parts.get(3).unwrap_or(&"").to_string();
        let recording_additional = parts.get(4..).map(|s| s.join(" ")).unwrap_or_default();
        
        Ok((admin_code, technician, equipment, recording_additional))
    }
}
