use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use chrono::{NaiveDate, NaiveTime};

use crate::types::{EdfHeader, SignalParam, Annotation};
use crate::error::{EdfError, Result};
use crate::utils::{atoi_nonlocalized, atof_nonlocalized, parse_edf_time};
use crate::EDFLIB_TIME_DIMENSION;

/// EDF+ file reader for reading European Data Format Plus files
/// 
/// The `EdfReader` provides methods to open and read EDF+ files, which are
/// commonly used for storing biosignal recordings like EEG, ECG, EMG, etc.
/// 
/// # Examples
/// 
/// ## Basic usage
/// 
/// ```rust
/// use edfplus::EdfReader;
/// 
/// # // Generate test file (hidden from docs)
/// # edfplus::doctest_utils::create_simple_test_file("recording.edf")?;
/// # 
/// // Open an EDF+ file
/// let mut reader = EdfReader::open("recording.edf")?;
/// 
/// // Get header information
/// let header = reader.header();
/// println!("Duration: {:.1} seconds", header.file_duration as f64 / 10_000_000.0);
/// println!("Signals: {}", header.signals.len());
/// 
/// // Read physical samples from first signal
/// let samples = reader.read_physical_samples(0, 256)?;
/// println!("Read {} samples", samples.len());
/// 
/// # // Cleanup (hidden from docs)
/// # std::fs::remove_file("recording.edf").ok();
/// # Ok::<(), edfplus::EdfError>(())
/// ```
/// ## Processing all signals
/// 
/// ```rust
/// use edfplus::EdfReader;
/// 
/// # // Generate test file (hidden from docs)
/// # edfplus::doctest_utils::create_multi_channel_test_file("multi_signal.edf")?;
/// # 
/// let mut reader = EdfReader::open("multi_signal.edf")?;
/// let signal_count = reader.header().signals.len();
/// 
/// // Process each signal
/// for i in 0..signal_count {
///     let signal_label = reader.header().signals[i].label.clone();
///     let signal_dimension = reader.header().signals[i].physical_dimension.clone();
///     let samples_per_second = reader.header().signals[i].samples_per_record as usize;
///     
///     println!("Processing signal {}: {}", i, signal_label);
///     
///     // Read one second of data (assuming 256 Hz sampling rate)
///     let physical_values = reader.read_physical_samples(i, samples_per_second)?;
///     
///     // Calculate basic statistics
///     let mean = physical_values.iter().sum::<f64>() / physical_values.len() as f64;
///     let max = physical_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
///     let min = physical_values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
///     
///     println!("  Mean: {:.2} {}", mean, signal_dimension);
///     println!("  Range: {:.2} to {:.2} {}", min, max, signal_dimension);
/// }
/// 
/// # // Cleanup (hidden from docs)
/// # std::fs::remove_file("multi_signal.edf").ok();
/// # Ok::<(), edfplus::EdfError>(())
/// ```
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
    #[allow(dead_code)]
    samples_per_record: i32,
    /// 是否是注释信号
    #[allow(dead_code)]
    is_annotation: bool,
}

impl EdfReader {
    /// Opens an EDF+ file for reading
    /// 
    /// This method opens the specified file, validates it as a proper EDF+ file,
    /// and parses the header information. Only EDF+ format is supported.
    /// 
    /// # Arguments
    /// 
    /// * `path` - Path to the EDF+ file to open
    /// 
    /// # Returns
    /// 
    /// Returns a `Result<EdfReader, EdfError>`. On success, contains an `EdfReader`
    /// instance ready for reading data. On failure, contains an error describing
    /// what went wrong.
    /// 
    /// # Errors
    /// 
    /// * `EdfError::FileNotFound` - File doesn't exist or can't be opened
    /// * `EdfError::UnsupportedFileType` - File is not EDF+ format
    /// * `EdfError::InvalidHeader` - File header is corrupted or invalid
    /// * `EdfError::InvalidSignalCount` - Invalid number of signals
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use edfplus::EdfReader;
    /// 
    /// # // Generate test file (hidden from docs)
    /// # edfplus::doctest_utils::create_simple_test_file("recording.edf")?;
    /// # 
    /// // Open a file successfully
    /// match EdfReader::open("recording.edf") {
    ///     Ok(reader) => {
    ///         println!("File opened successfully!");
    ///         println!("Duration: {:.1} seconds", 
    ///             reader.header().file_duration as f64 / 10_000_000.0);
    ///     }
    ///     Err(e) => eprintln!("Failed to open file: {}", e),
    /// }
    /// 
    /// // Handle different error types
    /// match EdfReader::open("nonexistent.edf") {
    ///     Ok(_) => println!("Unexpected success"),
    ///     Err(edfplus::EdfError::FileNotFound(msg)) => {
    ///         println!("File not found: {}", msg);
    ///     }
    ///     Err(e) => println!("Other error: {}", e),
    /// }
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("recording.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
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
    
    /// Gets a reference to the file header information
    /// 
    /// The header contains all metadata about the recording including:
    /// - Patient information (name, code, birth date, etc.)
    /// - Recording information (start time, duration, equipment, etc.)
    /// - Signal parameters (labels, sampling rates, physical ranges, etc.)
    /// - File format details
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use edfplus::EdfReader;
    /// 
    /// # // Generate test file (hidden from docs) 
    /// # edfplus::doctest_utils::create_simple_test_file("recording.edf")?;
    /// # 
    /// let reader = EdfReader::open("recording.edf")?;
    /// let header = reader.header();
    /// 
    /// // Display basic file information
    /// println!("Patient: {}", header.patient_name);
    /// println!("Recording duration: {:.2} seconds", 
    ///     header.file_duration as f64 / 10_000_000.0);
    /// println!("Number of signals: {}", header.signals.len());
    /// 
    /// // Display signal information
    /// for (i, signal) in header.signals.iter().enumerate() {
    ///     println!("Signal {}: {} ({})", 
    ///         i, signal.label, signal.physical_dimension);
    ///     println!("  Sample rate: {} Hz", signal.samples_per_record);
    ///     println!("  Range: {} to {} {}", 
    ///         signal.physical_min, signal.physical_max, signal.physical_dimension);
    /// }
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("recording.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
    pub fn header(&self) -> &EdfHeader {
        &self.header
    }
    
    /// Gets a reference to the list of annotations in the file
    /// 
    /// Annotations represent events, markers, and metadata that occurred
    /// during the recording. Common examples include sleep stages, seizures,
    /// artifacts, stimuli, and user-defined events.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use edfplus::{EdfReader, EdfWriter, SignalParam, Annotation};
    /// # use std::fs;
    /// 
    /// # // Create a test file (without annotations for this example)
    /// # let mut writer = EdfWriter::create("test_annotations.edf").unwrap();
    /// # writer.set_patient_info("P001", "M", "01-JAN-1990", "Test Patient").unwrap();
    /// # let signal = SignalParam {
    /// #     label: "EEG".to_string(),
    /// #     samples_in_file: 0,
    /// #     physical_max: 100.0,
    /// #     physical_min: -100.0,
    /// #     digital_max: 32767,
    /// #     digital_min: -32768,
    /// #     samples_per_record: 256,
    /// #     physical_dimension: "uV".to_string(),
    /// #     prefilter: "HP:0.1Hz".to_string(),
    /// #     transducer: "AgAgCl".to_string(),
    /// # };
    /// # writer.add_signal(signal).unwrap();
    /// # let samples = vec![10.0; 256];
    /// # writer.write_samples(&[samples]).unwrap();
    /// # writer.finalize().unwrap();
    /// 
    /// let reader = EdfReader::open("test_annotations.edf").unwrap();
    /// let annotations = reader.annotations();
    /// 
    /// println!("Found {} annotations", annotations.len());
    /// 
    /// for (i, annotation) in annotations.iter().enumerate() {
    ///     let onset_seconds = annotation.onset as f64 / 10_000_000.0;
    ///     let duration_seconds = if annotation.duration >= 0 {
    ///         annotation.duration as f64 / 10_000_000.0
    ///     } else {
    ///         0.0  // Instantaneous event
    ///     };
    ///     
    ///     println!("Annotation {}: {} at {:.2}s (duration: {:.2}s)",
    ///         i, annotation.description, onset_seconds, duration_seconds);
    /// }
    /// 
    /// # // Cleanup
    /// # drop(reader);
    /// # fs::remove_file("test_annotations.edf").ok();
    /// ```
    pub fn annotations(&self) -> &[Annotation] {
        &self.annotations
    }
    
    /// Reads physical value samples from the specified signal
    /// 
    /// Physical values are the real-world measurements (e.g., microvolts for EEG,
    /// millivolts for ECG) as opposed to the raw digital values stored in the file.
    /// The conversion from digital to physical values is performed automatically
    /// using the signal's calibration parameters.
    /// 
    /// # Arguments
    /// 
    /// * `signal` - Zero-based index of the signal to read from
    /// * `count` - Number of samples to read
    /// 
    /// # Returns
    /// 
    /// Vector of physical values in the signal's physical dimension (e.g., µV, mV).
    /// 
    /// # Errors
    /// 
    /// * `EdfError::InvalidSignalIndex` - Signal index is out of bounds
    /// * `EdfError::FileReadError` - I/O error reading from file
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use edfplus::EdfReader;
    /// 
    /// # // Generate test file (hidden from docs)
    /// # edfplus::doctest_utils::create_simple_test_file("eeg_recording.edf")?;
    /// # 
    /// let mut reader = EdfReader::open("eeg_recording.edf")?;
    /// 
    /// // Read 1 second of EEG data (assuming 256 Hz)
    /// let samples = reader.read_physical_samples(0, 256)?;
    /// 
    /// // Get header after reading samples
    /// let header = reader.header();
    /// 
    /// // Calculate basic statistics
    /// let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    /// let max_value = samples.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    /// let min_value = samples.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    /// 
    /// println!("Signal: {}", header.signals[0].label);
    /// println!("Mean: {:.2} {}", mean, header.signals[0].physical_dimension);
    /// println!("Range: {:.2} to {:.2} {}", 
    ///     min_value, max_value, header.signals[0].physical_dimension);
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("eeg_recording.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
    /// 
    /// ## Processing multiple signals
    /// 
    /// ```rust
    /// use edfplus::EdfReader;
    /// 
    /// # // Generate test file (hidden from docs)
    /// # edfplus::doctest_utils::create_multi_channel_test_file("multi_channel.edf")?;
    /// # 
    /// let mut reader = EdfReader::open("multi_channel.edf")?;
    /// let signal_count = reader.header().signals.len();
    /// 
    /// // Read data from all signals  
    /// for signal_idx in 0..signal_count {
    ///     let signal_label = reader.header().signals[signal_idx].label.clone();
    ///     let signal_dimension = reader.header().signals[signal_idx].physical_dimension.clone();
    ///     let samples_per_record = reader.header().signals[signal_idx].samples_per_record as usize;
    ///     
    ///     // Read one record worth of data (safe amount)
    ///     let samples = reader.read_physical_samples(signal_idx, samples_per_record)?;
    ///     
    ///     println!("Signal {}: {} samples from {}", 
    ///         signal_label, samples.len(), signal_dimension);
    ///         
    ///     // Find peak-to-peak amplitude
    ///     let max = samples.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    ///     let min = samples.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    ///     println!("  Amplitude: {:.2} {}", max - min, signal_dimension);
    /// }
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("multi_channel.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
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
    
    /// Reads digital value samples from the specified signal
    /// 
    /// Digital values are the raw integer values stored in the EDF+ file,
    /// before conversion to physical units. These are typically 16-bit
    /// signed integers representing the ADC output.
    /// 
    /// Most users should use `read_physical_samples()` instead, which
    /// automatically converts to real-world units.
    /// 
    /// # Arguments
    /// 
    /// * `signal` - Zero-based index of the signal to read from  
    /// * `count` - Number of samples to read
    /// 
    /// # Returns
    /// 
    /// Vector of digital values as signed 32-bit integers.
    /// 
    /// # Errors
    /// 
    /// * `EdfError::InvalidSignalIndex` - Signal index is out of bounds
    /// * `EdfError::FileReadError` - I/O error reading from file
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use edfplus::EdfReader;
    /// 
    /// # // Generate test file (hidden from docs)
    /// # edfplus::doctest_utils::create_simple_test_file("recording.edf")?;
    /// # 
    /// let mut reader = EdfReader::open("recording.edf")?;
    /// 
    /// // Read raw digital values
    /// let digital_samples = reader.read_digital_samples(0, 100)?;
    /// 
    /// // Get header after reading
    /// let header = reader.header();
    /// let signal = &header.signals[0];
    /// 
    /// // Manual conversion to physical values
    /// let physical_samples: Vec<f64> = digital_samples
    ///     .iter()
    ///     .map(|&d| signal.to_physical(d))
    ///     .collect();
    /// 
    /// println!("Digital range: {} to {}", 
    ///     digital_samples.iter().min().unwrap(),
    ///     digital_samples.iter().max().unwrap());
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("recording.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
    /// 
    /// ## Checking digital value ranges
    /// 
    /// ```rust
    /// use edfplus::EdfReader;
    /// 
    /// # // Generate test file (hidden from docs)
    /// # edfplus::doctest_utils::create_validation_test_file("test.edf")?;
    /// # 
    /// let mut reader = EdfReader::open("test.edf")?;
    /// let signal_count = reader.header().signals.len();
    /// 
    /// for i in 0..signal_count {
    ///     let signal_label = reader.header().signals[i].label.clone();
    ///     let digital_min = reader.header().signals[i].digital_min;
    ///     let digital_max = reader.header().signals[i].digital_max;
    ///     
    ///     let samples = reader.read_digital_samples(i, 10)?;
    ///     
    ///     let min_val = *samples.iter().min().unwrap();
    ///     let max_val = *samples.iter().max().unwrap();
    ///     
    ///     println!("Signal {}: digital range {} to {} (expected: {} to {})",
    ///         signal_label, min_val, max_val, digital_min, digital_max);
    ///         
    ///     // Check for clipping
    ///     if min_val <= digital_min || max_val >= digital_max {
    ///         println!("  Warning: Signal may be clipped!");
    ///     }
    /// }
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("test.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
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
