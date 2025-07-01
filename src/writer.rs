use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use chrono::{NaiveDate, NaiveTime, Datelike, Timelike};

use crate::types::SignalParam;
use crate::error::{EdfError, Result};
use crate::EDFLIB_TIME_DIMENSION;

/// EDF+ file writer for creating European Data Format Plus files
/// 
/// The `EdfWriter` provides methods to create new EDF+ files and write
/// biosignal data with proper metadata. It ensures compliance with the
/// EDF+ specification and provides validation of signal parameters.
/// 
/// # File Creation Workflow
/// 
/// 1. Create writer with `EdfWriter::create()`
/// 2. Set patient and recording information
/// 3. Add signal definitions with `add_signal()`
/// 4. Write sample data with `write_samples()`
/// 5. Finalize the file with `finalize()`
/// 
/// # Examples
/// 
/// ## Basic EDF+ file creation
/// 
/// ```rust
/// use edfplus::{EdfWriter, SignalParam};
/// 
/// // Create new EDF+ file
/// let mut writer = EdfWriter::create("output.edf")?;
/// 
/// // Set patient information
/// writer.set_patient_info("P001", "M", "01-JAN-1990", "Test Patient")?;
/// 
/// // Define an EEG signal
/// let eeg_signal = SignalParam {
///     label: "EEG Fp1".to_string(),
///     samples_in_file: 0,  // Calculated automatically
///     physical_max: 200.0,
///     physical_min: -200.0,
///     digital_max: 32767,
///     digital_min: -32768,
///     samples_per_record: 256,  // 256 Hz sampling rate
///     physical_dimension: "uV".to_string(),
///     prefilter: "HP:0.1Hz LP:70Hz".to_string(),
///     transducer: "AgAgCl cup electrodes".to_string(),
/// };
/// 
/// writer.add_signal(eeg_signal)?;
/// 
/// // Generate and write sample data
/// let mut samples = Vec::new();
/// for i in 0..256 {
///     let t = i as f64 / 256.0;
///     let value = 50.0 * (2.0 * std::f64::consts::PI * 10.0 * t).sin();
///     samples.push(value);
/// }
/// 
/// writer.write_samples(&[samples])?;
/// writer.finalize()?;
/// 
/// # // Cleanup (hidden from docs)
/// # std::fs::remove_file("output.edf").ok();
/// # Ok::<(), edfplus::EdfError>(())
/// ```
/// 
/// ## Multi-channel recording
/// 
/// ```rust
/// use edfplus::{EdfWriter, SignalParam};
/// 
/// # use edfplus::doctest_utils::*;
/// let mut writer = EdfWriter::create("multi_channel.edf")?;
/// writer.set_patient_info("P002", "F", "15-MAR-1985", "Multi Channel Test")?;
/// 
/// // Add multiple signals
/// let signals = vec![
///     SignalParam {
///         label: "EEG C3".to_string(),
///         samples_in_file: 0,
///         physical_max: 200.0, physical_min: -200.0,
///         digital_max: 32767, digital_min: -32768,
///         samples_per_record: 256,
///         physical_dimension: "uV".to_string(),
///         prefilter: "HP:0.1Hz LP:70Hz".to_string(),
///         transducer: "AgAgCl electrodes".to_string(),
///     },
///     SignalParam {
///         label: "ECG Lead II".to_string(),
///         samples_in_file: 0,
///         physical_max: 5.0, physical_min: -5.0,
///         digital_max: 32767, digital_min: -32768,
///         samples_per_record: 256,
///         physical_dimension: "mV".to_string(),
///         prefilter: "HP:0.1Hz LP:100Hz".to_string(),
///         transducer: "Chest electrodes".to_string(),
///     },
/// ];
/// 
/// for signal in signals {
///     writer.add_signal(signal)?;
/// }
/// 
/// // Write 10 seconds of data
/// for second in 0..10 {
///     let mut eeg_samples = Vec::new();
///     let mut ecg_samples = Vec::new();
///     
///     for i in 0..256 {
///         let t = (second * 256 + i) as f64 / 256.0;
///         
///         // EEG: Alpha wave (10 Hz) with noise
///         let eeg = 30.0 * (2.0 * std::f64::consts::PI * 10.0 * t).sin()
///                 + 5.0 * (2.0 * std::f64::consts::PI * 50.0 * t).sin();
///         eeg_samples.push(eeg);
///         
///         // ECG: Heart beat pattern (60 BPM)
///         let ecg = 2.0 * (2.0 * std::f64::consts::PI * 1.0 * t).sin();
///         ecg_samples.push(ecg);
///     }
///     
///     writer.write_samples(&[eeg_samples, ecg_samples])?;
/// }
/// 
/// writer.finalize()?;
/// 
/// # // Cleanup (hidden from docs)
/// # std::fs::remove_file("multi_channel.edf").ok();
/// # Ok::<(), edfplus::EdfError>(())
/// ```
pub struct EdfWriter {
    file: BufWriter<File>,
    signals: Vec<SignalParam>,
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
    
    // 注释存储
    annotations: Vec<crate::types::Annotation>,

    // 子秒开始时间
    starttime_subsecond: i64,
}

impl EdfWriter {
    /// Creates a new EDF+ file writer
    /// 
    /// Opens a new file for writing and initializes the writer with default
    /// values. The file will be created (or truncated if it exists).
    /// 
    /// # Arguments
    /// 
    /// * `path` - Path where the EDF+ file should be created
    /// 
    /// # Returns
    /// 
    /// Returns a `Result<EdfWriter, EdfError>`. On success, contains an `EdfWriter`
    /// ready for configuration and data writing.
    /// 
    /// # Errors
    /// 
    /// * `EdfError::FileNotFound` - Cannot create file (permission issues, invalid path, etc.)
    /// 
    /// # Default Values
    /// 
    /// The writer is initialized with the following defaults:
    /// - Start date: January 1, 1985
    /// - Start time: 00:00:00
    /// - All patient and recording fields set to "X" (anonymized)
    /// - Data record duration: 1 second
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use edfplus::EdfWriter;
    /// 
    /// // Create a new EDF+ file
    /// let writer = EdfWriter::create("new_recording.edf")?;
    /// println!("EDF+ writer created successfully");
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("new_recording.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
    /// 
    /// ## Handling creation errors
    /// 
    /// ```rust
    /// use edfplus::EdfWriter;
    /// 
    /// match EdfWriter::create("/invalid/path/file.edf") {
    ///     Ok(_) => println!("File created"),
    ///     Err(e) => eprintln!("Failed to create file: {}", e),
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
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
            annotations: Vec::new(),
            starttime_subsecond: 0,
        })
    }
    
    /// Adds a signal definition to the EDF+ file
    /// 
    /// Each signal represents a data channel (e.g., EEG electrode, ECG lead).
    /// Signals must be added before writing any data. The order in which
    /// signals are added determines their index for data writing.
    /// 
    /// # Arguments
    /// 
    /// * `signal` - SignalParam containing all signal metadata
    /// 
    /// # Errors
    /// 
    /// * `EdfError::InvalidFormat` - Trying to add signal after header is written
    /// * `EdfError::PhysicalMinEqualsMax` - Invalid physical range
    /// * `EdfError::DigitalMinEqualsMax` - Invalid digital range
    /// 
    /// # Signal Parameter Requirements
    /// 
    /// - `physical_min` must be different from `physical_max`
    /// - `digital_min` must be different from `digital_max`
    /// - `samples_per_record` should match the intended sampling rate
    /// - `label` should be descriptive and follow EDF+ conventions
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use edfplus::{EdfWriter, SignalParam};
    /// 
    /// let mut writer = EdfWriter::create("signals.edf")?;
    /// 
    /// // Add an EEG signal
    /// let eeg_signal = SignalParam {
    ///     label: "EEG Fp1-A1".to_string(),
    ///     samples_in_file: 0,  // Will be calculated
    ///     physical_max: 200.0,    // +200 µV
    ///     physical_min: -200.0,   // -200 µV  
    ///     digital_max: 32767,     // 16-bit signed max
    ///     digital_min: -32768,    // 16-bit signed min
    ///     samples_per_record: 256, // 256 Hz sampling
    ///     physical_dimension: "uV".to_string(),
    ///     prefilter: "HP:0.1Hz LP:70Hz N:50Hz".to_string(),
    ///     transducer: "AgAgCl cup electrodes".to_string(),
    /// };
    /// 
    /// writer.add_signal(eeg_signal)?;
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("signals.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
    /// 
    /// ## Adding multiple signals with different sampling rates
    /// 
    /// ```rust
    /// use edfplus::{EdfWriter, SignalParam};
    /// 
    /// let mut writer = EdfWriter::create("mixed_rates.edf")?;
    /// 
    /// // High-frequency EEG signal
    /// writer.add_signal(SignalParam {
    ///     label: "EEG C3-A1".to_string(),
    ///     samples_in_file: 0,
    ///     physical_max: 200.0, physical_min: -200.0,
    ///     digital_max: 32767, digital_min: -32768,
    ///     samples_per_record: 512,  // 512 Hz
    ///     physical_dimension: "uV".to_string(),
    ///     prefilter: "HP:0.1Hz LP:200Hz".to_string(),
    ///     transducer: "Gold cup electrodes".to_string(),
    /// })?;
    /// 
    /// // Lower-frequency physiological signal
    /// writer.add_signal(SignalParam {
    ///     label: "Temperature".to_string(),
    ///     samples_in_file: 0,
    ///     physical_max: 40.0, physical_min: 30.0,
    ///     digital_max: 32767, digital_min: -32768,
    ///     samples_per_record: 1,    // 1 Hz
    ///     physical_dimension: "degC".to_string(),
    ///     prefilter: "None".to_string(),
    ///     transducer: "Thermistor".to_string(),
    /// })?;
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("mixed_rates.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
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
    
    /// Sets patient information for the EDF+ file
    /// 
    /// Patient information is embedded in the EDF+ header and follows
    /// specific formatting requirements. This information is crucial
    /// for medical applications but can be anonymized for privacy.
    /// 
    /// # Arguments
    /// 
    /// * `code` - Patient identification code (max 80 chars)
    /// * `sex` - Patient sex: "M", "F", or "X" (unknown)
    /// * `birthdate` - Birth date in DD-MMM-YYYY format or "X"
    /// * `name` - Patient name or "X" for anonymized data
    /// 
    /// # Errors
    /// 
    /// * `EdfError::InvalidFormat` - Trying to modify after header written
    /// 
    /// # Format Requirements
    /// 
    /// - Patient code should be unique and meaningful
    /// - Sex must be "M", "F", or "X"
    /// - Birth date format: "02-MAY-1951" or "X" if unknown
    /// - Name can be full name or "X" for anonymization
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use edfplus::EdfWriter;
    /// 
    /// let mut writer = EdfWriter::create("patient_data.edf")?;
    /// 
    /// // Set complete patient information
    /// writer.set_patient_info(
    ///     "P001-2024",           // Patient code
    ///     "F",                   // Female
    ///     "15-MAR-1990",         // Birth date
    ///     "Jane Doe"             // Patient name
    /// )?;
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("patient_data.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
    /// 
    /// ## Anonymized patient data
    /// 
    /// ```rust
    /// use edfplus::EdfWriter;
    /// 
    /// let mut writer = EdfWriter::create("anonymous.edf")?;
    /// 
    /// // Anonymized information for privacy protection
    /// writer.set_patient_info(
    ///     "ANON-001",    // Anonymous code
    ///     "X",           // Sex unknown/anonymized
    ///     "X",           // Birth date anonymized
    ///     "X"            // Name anonymized
    /// )?;
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("anonymous.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
    /// 
    /// ## Research study format
    /// 
    /// ```rust
    /// use edfplus::EdfWriter;
    /// 
    /// let mut writer = EdfWriter::create("study_subject.edf")?;
    /// 
    /// // Research study patient coding
    /// writer.set_patient_info(
    ///     "STUDY-EEG-S042",      // Study-specific ID
    ///     "M",                   // Male
    ///     "22-JUL-1985",         // Known birth date
    ///     "Subject 042"          // Study identifier
    /// )?;
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("study_subject.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
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
    
    /// Sets the data record duration for the EDF+ file
    /// 
    /// The data record duration determines how long each data record represents
    /// in time. This affects the temporal resolution and file organization.
    /// Most EDF+ files use 1 second data records, but other durations are possible.
    /// 
    /// # Arguments
    /// 
    /// * `duration_seconds` - Duration of each data record in seconds
    /// 
    /// # Errors
    /// 
    /// * `EdfError::InvalidFormat` - Trying to modify after header written
    /// * `EdfError::InvalidArgument` - Duration <= 0 or too large
    /// 
    /// # Common Values
    /// 
    /// - 1.0 seconds: Standard for most clinical recordings
    /// - 0.1 seconds: Higher temporal resolution for fast events
    /// - 10.0 seconds: Lower resolution for long-term monitoring
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use edfplus::EdfWriter;
    /// 
    /// let mut writer = EdfWriter::create("custom_duration.edf")?;
    /// 
    /// // Set 0.5 second data records for higher temporal resolution
    /// writer.set_datarecord_duration(0.5)?;
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("custom_duration.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
    /// 
    /// ## High-frequency recording
    /// 
    /// ```rust
    /// use edfplus::EdfWriter;
    /// 
    /// let mut writer = EdfWriter::create("high_freq.edf")?;
    /// 
    /// // Use 0.1 second records for fast neural signals
    /// writer.set_datarecord_duration(0.1)?;
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("high_freq.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
    pub fn set_datarecord_duration(&mut self, duration_seconds: f64) -> Result<()> {
        if self.header_written {
            return Err(EdfError::InvalidFormat("Cannot modify data record duration after writing header".to_string()));
        }
        
        if duration_seconds <= 0.0 || duration_seconds > 3600.0 {
            return Err(EdfError::InvalidFormat("Data record duration must be between 0 and 3600 seconds".to_string()));
        }
        
        // Convert seconds to EDFLIB_TIME_DIMENSION units (100 nanoseconds)
        self.datarecord_duration = (duration_seconds * EDFLIB_TIME_DIMENSION as f64) as i64;
        Ok(())
    }

    /// 写入头部
    fn write_header(&mut self, total_datarecords: i64) -> Result<()> {
        if self.header_written {
            return Ok(());
        }
        
        // 添加注释信号
        // 根据 EDF+ 规范，注释信号需要足够空间存储 TAL 数据
        // EDFLIB_ANNOTATION_BYTES = 120, 每样本2字节，所以需要60个样本
        let annotation_bytes_per_record = 120; // 与 edflib 兼容
        let annotation_samples_per_record = annotation_bytes_per_record / 2; // 每样本2字节
        
        let annotation_signal = SignalParam {
            label: "EDF Annotations ".to_string(), // 注意末尾的空格
            samples_in_file: total_datarecords * annotation_samples_per_record as i64,
            physical_max: 1.0,
            physical_min: -1.0,
            digital_max: 32767,
            digital_min: -32768,
            samples_per_record: annotation_samples_per_record as i32,
            physical_dimension: "".to_string(),
            prefilter: "".to_string(),
            transducer: "".to_string(),
        };
        
        let total_signals = self.signals.len() + 1; // +1 for annotation
        let header_size = (total_signals + 1) * 256;
        
        // 写入主头部 (256字节) - 按照edflib格式
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
        let duration_seconds = self.datarecord_duration as f64 / EDFLIB_TIME_DIMENSION as f64;
        let duration_str = format!("{:<8}", duration_seconds);
        main_header[244..252].copy_from_slice(duration_str.as_bytes());
        
        // 信号数 (4字节)
        let signals_str = format!("{:<4}", total_signals);
        main_header[252..256].copy_from_slice(signals_str.as_bytes());
        
        self.file.write_all(&main_header)?;
        
        // 写入信号头部
        self.write_signal_headers(&annotation_signal)?;
        
        self.header_written = true;
        Ok(())
    }
    
    /// Writes physical sample data to the EDF+ file
    /// 
    /// Writes one data record worth of samples for all signals. Each signal
    /// must provide the same number of samples. Physical values are automatically
    /// converted to digital values using each signal's calibration parameters.
    /// 
    /// # Arguments
    /// 
    /// * `samples` - Vector of sample vectors, one per signal in order
    /// 
    /// # Errors
    /// 
    /// * `EdfError::InvalidFormat` - Mismatched sample counts between signals
    /// * `EdfError::FileWriteError` - I/O error writing to file
    /// 
    /// # Sample Organization
    /// 
    /// The `samples` parameter must be organized as:
    /// - Outer vector: one element per signal (in order added)
    /// - Inner vectors: physical values for each signal
    /// - All inner vectors must have the same length
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use edfplus::{EdfWriter, SignalParam};
    /// 
    /// let mut writer = EdfWriter::create("samples.edf")?;
    /// 
    /// // Add two signals
    /// writer.add_signal(SignalParam {
    ///     label: "EEG".to_string(),
    ///     samples_in_file: 0,
    ///     physical_max: 100.0, physical_min: -100.0,
    ///     digital_max: 32767, digital_min: -32768,
    ///     samples_per_record: 256,
    ///     physical_dimension: "uV".to_string(),
    ///     prefilter: "None".to_string(),
    ///     transducer: "Electrode".to_string(),
    /// })?;
    /// 
    /// writer.add_signal(SignalParam {
    ///     label: "ECG".to_string(),
    ///     samples_in_file: 0,
    ///     physical_max: 5.0, physical_min: -5.0,
    ///     digital_max: 32767, digital_min: -32768,
    ///     samples_per_record: 256,
    ///     physical_dimension: "mV".to_string(),
    ///     prefilter: "None".to_string(),
    ///     transducer: "Electrode".to_string(),
    /// })?;
    /// 
    /// // Generate sample data (256 samples per signal)
    /// let mut eeg_samples = Vec::new();
    /// let mut ecg_samples = Vec::new();
    /// 
    /// for i in 0..256 {
    ///     let t = i as f64 / 256.0;
    ///     eeg_samples.push(20.0 * (2.0 * std::f64::consts::PI * 10.0 * t).sin());
    ///     ecg_samples.push(1.0 * (2.0 * std::f64::consts::PI * 1.0 * t).sin());
    /// }
    /// 
    /// // Write the samples (order matches signal addition order)
    /// writer.write_samples(&[eeg_samples, ecg_samples])?;
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("samples.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
    /// 
    /// ## Writing multiple data records
    /// 
    /// ```rust
    /// use edfplus::{EdfWriter, SignalParam};
    /// 
    /// let mut writer = EdfWriter::create("continuous.edf")?;
    /// 
    /// // Add a signal
    /// writer.add_signal(SignalParam {
    ///     label: "Continuous Signal".to_string(),
    ///     samples_in_file: 0,
    ///     physical_max: 1.0, physical_min: -1.0,
    ///     digital_max: 32767, digital_min: -32768,
    ///     samples_per_record: 100,  // 100 Hz
    ///     physical_dimension: "V".to_string(),
    ///     prefilter: "None".to_string(),
    ///     transducer: "Sensor".to_string(),
    /// })?;
    /// 
    /// // Write 10 seconds of data (10 data records)
    /// for second in 0..10 {
    ///     let mut samples = Vec::new();
    ///     
    ///     for i in 0..100 {
    ///         let t = (second * 100 + i) as f64 / 100.0;
    ///         let value = (2.0 * std::f64::consts::PI * 2.0 * t).sin();
    ///         samples.push(value);
    ///     }
    ///     
    ///     writer.write_samples(&[samples])?;
    /// }
    /// 
    /// writer.finalize()?;
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("continuous.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
    pub fn write_samples(&mut self, samples: &[Vec<f64>]) -> Result<()> {
        if samples.len() != self.signals.len() {
            return Err(EdfError::InvalidFormat("Sample count must match signal count".to_string()));
        }
        
        // 验证每个信号的样本数
        for (i, signal_samples) in samples.iter().enumerate() {
            let expected_samples = self.signals[i].samples_per_record as usize;
            if signal_samples.len() != expected_samples {
                return Err(EdfError::InvalidFormat(
                    format!("Signal {} expected {} samples per record, got {}", 
                           i, expected_samples, signal_samples.len())
                ));
            }
        }
        
        // 如果还没写头部，先写头部
        if !self.header_written {
            self.write_header(1)?; // 临时使用1，会在finalize时更新
        }
        
        // 按照edflib的顺序写入数据：信号数据 + 注释信号
    
        // 写入所有信号的样本数据
        for signal_idx in 0..self.signals.len() {
            let signal = &self.signals[signal_idx];
            let signal_samples = &samples[signal_idx];
            
            for &physical_value in signal_samples {
                let digital_value = signal.to_digital(physical_value);
                
                // 应用范围限制
                let clamped_value = digital_value
                    .max(signal.digital_min)
                    .min(signal.digital_max);
                
                // 写入为16位小端序
                let bytes = (clamped_value as i16).to_le_bytes();
                self.file.write_all(&bytes)?;
            }
        }
        
        // 写入注释信号的TAL数据
        let annotation_data = self.generate_annotation_tal(self.samples_written)?;
        self.file.write_all(&annotation_data)?;
        
        self.samples_written += 1;
        Ok(())
    }
    
    /// Finalizes the EDF+ file and closes it
    /// 
    /// This method must be called to complete the file writing process.
    /// It flushes any remaining data to disk and properly closes the file.
    /// After calling this method, the writer is consumed and cannot be used again.
    /// 
    /// # Errors
    /// 
    /// * `EdfError::FileWriteError` - I/O error during file finalization
    /// 
    /// # File Integrity
    /// 
    /// Failing to call `finalize()` may result in:
    /// - Incomplete file headers
    /// - Missing data records
    /// - Corrupted file structure
    /// 
    /// Always call `finalize()` when finished writing data.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use edfplus::{EdfWriter, SignalParam};
    /// 
    /// let mut writer = EdfWriter::create("final_test.edf")?;
    /// 
    /// // Add signal and write data...
    /// writer.add_signal(SignalParam {
    ///     label: "Test Signal".to_string(),
    ///     samples_in_file: 0,
    ///     physical_max: 1.0, physical_min: -1.0,
    ///     digital_max: 32767, digital_min: -32768,
    ///     samples_per_record: 10,
    ///     physical_dimension: "V".to_string(),
    ///     prefilter: "None".to_string(),
    ///     transducer: "Test".to_string(),
    /// })?;
    /// 
    /// let samples = vec![0.1, 0.2, 0.3, 0.4, 0.5, -0.1, -0.2, -0.3, -0.4, -0.5];
    /// writer.write_samples(&[samples])?;
    /// 
    /// // Always finalize to ensure file integrity
    /// writer.finalize()?;
    /// 
    /// # // Cleanup (hidden from docs)
    /// # std::fs::remove_file("final_test.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
    /// 
    /// ## Error handling during finalization
    /// 
    /// ```rust
    /// use edfplus::{EdfWriter, SignalParam};
    /// 
    /// fn test_finalize() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut writer = EdfWriter::create("error_test.edf")?;
    ///     // ... add signals and write data ...
    ///     
    ///     match writer.finalize() {
    ///         Ok(()) => println!("File successfully completed"),
    ///         Err(e) => {
    ///             eprintln!("Error finalizing file: {}", e);
    ///             // File may be corrupted
    ///         }
    ///     }
    ///     
    ///     # // Cleanup (hidden from docs)
    ///     # std::fs::remove_file("error_test.edf").ok();
    ///     Ok(())
    /// }
    /// 
    /// # test_finalize().unwrap();
    /// ```
    pub fn finalize(mut self) -> Result<()> {
        // 如果有数据写入但头部记录数不正确，需要更新头部
        if self.header_written && self.samples_written > 1 {
            use std::io::{Seek, SeekFrom};
            
            // 刷新缓冲区以确保所有数据都写入了
            self.file.flush()?;
            
            // 获取内部文件引用并seek到数据记录数位置 (236-244字节)
            let mut file = self.file.into_inner().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            file.seek(SeekFrom::Start(236))?;
            
            // 更新数据记录数
            let datarecords_str = format!("{:<8}", self.samples_written);
            file.write_all(datarecords_str.as_bytes())?;
            
            // 确保数据写入磁盘
            file.flush()?;
        } else {
            self.file.flush()?;
        }
        Ok(())
    }
    
    /// Adds an annotation/event to the EDF+ file
    /// 
    /// Annotations allow you to mark events, triggers, or time periods within
    /// the recording. They are stored as part of the EDF+ format and can be
    /// read back later.
    /// 
    /// # Arguments
    /// 
    /// * `onset_seconds` - Time when the event occurred (seconds since recording start)
    /// * `duration_seconds` - Duration of the event in seconds (None for instantaneous events)
    /// * `description` - UTF-8 text describing the event
    /// 
    /// # Time Precision
    /// 
    /// Time values are internally stored with 100-nanosecond precision.
    /// Input values will be rounded to the nearest 100 nanoseconds.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use edfplus::{EdfWriter, SignalParam};
    /// # use std::fs;
    /// 
    /// let mut writer = EdfWriter::create("annotations_test.edf")?;
    /// writer.set_patient_info("P001", "M", "01-JAN-1990", "Test Patient")?;
    /// 
    /// // Add a signal
    /// let signal = SignalParam {
    ///     label: "EEG".to_string(),
    ///     samples_in_file: 0,
    ///     physical_max: 100.0,
    ///     physical_min: -100.0,
    ///     digital_max: 32767,
    ///     digital_min: -32768,
    ///     samples_per_record: 256,
    ///     physical_dimension: "uV".to_string(),
    ///     prefilter: "HP:0.1Hz".to_string(),
    ///     transducer: "AgAgCl".to_string(),
    /// };
    /// writer.add_signal(signal)?;
    /// 
    /// // Write some data
    /// let samples = vec![10.0; 256];
    /// writer.write_samples(&[samples])?;
    /// 
    /// // Add annotations
    /// writer.add_annotation(0.5, None, "Recording start")?;
    /// writer.add_annotation(2.0, Some(1.0), "Sleep stage 1")?;
    /// writer.add_annotation(5.5, None, "Eye movement")?;
    /// 
    /// writer.finalize()?;
    /// 
    /// # // Cleanup
    /// # std::fs::remove_file("annotations_test.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
    pub fn add_annotation(&mut self, onset_seconds: f64, duration_seconds: Option<f64>, description: &str) -> Result<()> {
        // Validate inputs
        if onset_seconds < 0.0 {
            return Err(EdfError::InvalidFormat("Annotation onset cannot be negative".to_string()));
        }
        
        if let Some(duration) = duration_seconds {
            if duration < 0.0 {
                return Err(EdfError::InvalidFormat("Annotation duration cannot be negative".to_string()));
            }
        }
        
        if description.is_empty() {
            return Err(EdfError::InvalidFormat("Annotation description cannot be empty".to_string()));
        }
        
        if description.len() > 512 {
            return Err(EdfError::InvalidFormat("Annotation description too long (max 512 characters)".to_string()));
        }
        
        // Convert to internal time units (100 nanoseconds)
        let onset = (onset_seconds * EDFLIB_TIME_DIMENSION as f64) as i64;
        let duration = duration_seconds
            .map(|d| (d * EDFLIB_TIME_DIMENSION as f64) as i64)
            .unwrap_or(-1);
        
        // Create and store annotation
        let annotation = crate::types::Annotation {
            onset,
            duration,
            description: description.to_string(),
        };
        
        self.annotations.push(annotation);
        Ok(())
    }

    /// Gets the current number of annotations
    /// 
    /// This can be useful for tracking how many annotations have been added
    /// before finalizing the file.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use edfplus::{EdfWriter, SignalParam};
    /// # use std::fs;
    /// 
    /// let mut writer = EdfWriter::create("count_test.edf")?;
    /// writer.set_patient_info("P001", "M", "01-JAN-1990", "Test")?;
    /// 
    /// // Initially no annotations
    /// assert_eq!(writer.annotation_count(), 0);
    /// 
    /// writer.add_annotation(1.0, None, "Event 1")?;
    /// assert_eq!(writer.annotation_count(), 1);
    /// 
    /// writer.add_annotation(2.0, Some(0.5), "Event 2")?;
    /// assert_eq!(writer.annotation_count(), 2);
    /// 
    /// # // Cleanup
    /// # std::fs::remove_file("count_test.edf").ok();
    /// # Ok::<(), edfplus::EdfError>(())
    /// ```
    pub fn annotation_count(&self) -> usize {
        self.annotations.len()
    }

    /// Generates TAL (Time-stamped Annotations Lists) format for annotations
    /// 
    /// This creates the EDF+ annotation signal data in the correct format.
    /// Each data record gets exactly 120 bytes for the annotation signal.
    fn generate_annotation_tal(&self, data_record_index: usize) -> Result<Vec<u8>> {
        const ANNOTATION_BYTES: usize = 120; // 与 edflib 兼容
        let mut tal_data = Vec::with_capacity(ANNOTATION_BYTES);
        
        // 数据记录的时间范围
        let data_record_time_start = data_record_index as f64 * (self.datarecord_duration as f64 / EDFLIB_TIME_DIMENSION as f64);
        let data_record_time_end = (data_record_index + 1) as f64 * (self.datarecord_duration as f64 / EDFLIB_TIME_DIMENSION as f64);
        
        // 第一个数据记录总是包含时间戳记录
        if data_record_index == 0 {
            // 格式: "+<onset>\x14\x14\x00"
            tal_data.push(b'+');
            let time_str = format!("{}", data_record_time_start);
            tal_data.extend_from_slice(time_str.as_bytes());
            tal_data.push(0x14); // ASCII 20 - start of annotation
            tal_data.push(0x14); // ASCII 20 - end of annotation (empty)
            tal_data.push(0x00); // Null terminator
        }
        
        // 添加落在此数据记录时间范围内的注释
        for annotation in &self.annotations {
            let annotation_time = annotation.onset as f64 / EDFLIB_TIME_DIMENSION as f64;
            
            // 检查注释是否属于当前数据记录
            if annotation_time >= data_record_time_start && annotation_time < data_record_time_end {
                // 确保有足够空间 (至少需要时间戳+描述+分隔符)
                let needed_space = annotation_time.to_string().len() + annotation.description.len() + 10;
                if tal_data.len() + needed_space > ANNOTATION_BYTES - 5 {
                    break; // 没有足够空间，跳过剩余注释
                }
                
                // 格式: "+<onset>[\x15<duration>]\x14<description>\x14"
                tal_data.push(b'+');
                let time_str = format!("{:.6}", annotation_time).trim_end_matches('0').trim_end_matches('.').to_string();
                tal_data.extend_from_slice(time_str.as_bytes());
                
                // 添加持续时间（如果指定）
                if annotation.duration >= 0 {
                    tal_data.push(0x15); // ASCII 21 - duration separator
                    let duration_str = format!("{:.6}", annotation.duration as f64 / EDFLIB_TIME_DIMENSION as f64)
                        .trim_end_matches('0').trim_end_matches('.').to_string();
                    tal_data.extend_from_slice(duration_str.as_bytes());
                }
                
                tal_data.push(0x14); // ASCII 20 - start of description
                tal_data.extend_from_slice(annotation.description.as_bytes());
                tal_data.push(0x14); // ASCII 20 - end of annotation
            }
        }
        
        // 填充到确切的 120 字节，用零填充
        tal_data.resize(ANNOTATION_BYTES, 0x00);
        Ok(tal_data)
    }

    // 添加subsecond开始时间支持
    pub fn set_subsecond_starttime(&mut self, subsecond: i64) -> Result<()> {
        if self.header_written {
            return Err(EdfError::InvalidFormat("Cannot modify subsecond start time after writing header".to_string()));
        }
        
        if subsecond < 0 || subsecond >= EDFLIB_TIME_DIMENSION {
            return Err(EdfError::InvalidFormat("Subsecond must be between 0 and 9999999".to_string()));
        }
        
        self.starttime_subsecond = subsecond;
        Ok(())
    }

    fn write_signal_headers(&mut self, annotation_signal: &SignalParam) -> Result<()> {
        let mut all_signals = self.signals.clone();
        all_signals.push(annotation_signal.clone());
        
        // 按照edflib的字段顺序写入，每个字段所有信号一起写
    
        // 1. 标签 (16字节 × 信号数)
        for signal in &all_signals {
            let mut field_data = [b' '; 16];
            let label_bytes = signal.label.as_bytes();
            let len = label_bytes.len().min(16);
            field_data[..len].copy_from_slice(&label_bytes[..len]);
            self.file.write_all(&field_data)?;
        }
        
        // 2. 传感器 (80字节 × 信号数)
        for signal in &all_signals {
            let mut field_data = [b' '; 80];
            let trans_bytes = signal.transducer.as_bytes();
            let len = trans_bytes.len().min(80);
            field_data[..len].copy_from_slice(&trans_bytes[..len]);
            self.file.write_all(&field_data)?;
        }
        
        // 3. 物理单位 (8字节 × 信号数)
        for signal in &all_signals {
            let mut field_data = [b' '; 8];
            let unit_bytes = signal.physical_dimension.as_bytes();
            let len = unit_bytes.len().min(8);
            field_data[..len].copy_from_slice(&unit_bytes[..len]);
            self.file.write_all(&field_data)?;
        }
        
        // 4. 物理最小值 (8字节 × 信号数)
        for signal in &all_signals {
            let phys_min_str = format!("{:<8}", signal.physical_min);
            self.file.write_all(phys_min_str.as_bytes())?;
        }
        
        // 5. 物理最大值 (8字节 × 信号数)
        for signal in &all_signals {
            let phys_max_str = format!("{:<8}", signal.physical_max);
            self.file.write_all(phys_max_str.as_bytes())?;
        }
        
        // 6. 数字最小值 (8字节 × 信号数)
        for signal in &all_signals {
            let dig_min_str = format!("{:<8}", signal.digital_min);
            self.file.write_all(dig_min_str.as_bytes())?;
        }
        
        // 7. 数字最大值 (8字节 × 信号数)
        for signal in &all_signals {
            let dig_max_str = format!("{:<8}", signal.digital_max);
            self.file.write_all(dig_max_str.as_bytes())?;
        }
        
        // 8. 预滤波 (80字节 × 信号数)
        for signal in &all_signals {
            let mut field_data = [b' '; 80];
            let prefilter_bytes = signal.prefilter.as_bytes();
            let len = prefilter_bytes.len().min(80);
            field_data[..len].copy_from_slice(&prefilter_bytes[..len]);
            self.file.write_all(&field_data)?;
        }
        
        // 9. 每记录样本数 (8字节 × 信号数)
        for signal in &all_signals {
            let samples_str = format!("{:<8}", signal.samples_per_record);
            self.file.write_all(samples_str.as_bytes())?;
        }
        
        // 10. 保留字段 (32字节 × 信号数)
        for _signal in &all_signals {
            let field_data = [b' '; 32];
            self.file.write_all(&field_data)?;
        }
        
        Ok(())
    }
}
