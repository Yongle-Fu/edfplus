use chrono::{NaiveDate, NaiveTime};

#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    EdfPlus,
}

#[derive(Debug, Clone)]
pub struct SignalParam {
    pub label: String,
    pub samples_in_file: i64,
    pub physical_max: f64,
    pub physical_min: f64,
    pub digital_max: i32,
    pub digital_min: i32,
    pub samples_per_record: i32,
    pub physical_dimension: String,
    pub prefilter: String,
    pub transducer: String,
}

impl SignalParam {
    /// 计算物理值转换参数
    pub fn bit_value(&self) -> f64 {
        (self.physical_max - self.physical_min) / 
        (self.digital_max - self.digital_min) as f64
    }
    
    /// 计算偏移量
    pub fn offset(&self) -> f64 {
        self.physical_max / self.bit_value() - self.digital_max as f64
    }
    
    /// 将数字值转换为物理值
    pub fn to_physical(&self, digital_value: i32) -> f64 {
        self.bit_value() * (self.offset() + digital_value as f64)
    }
    
    /// 将物理值转换为数字值
    pub fn to_digital(&self, physical_value: f64) -> i32 {
        let digital = (physical_value / self.bit_value()) - self.offset();
        digital.round() as i32
    }
}

#[derive(Debug, Clone)]
pub struct Annotation {
    pub onset: i64,           // 开始时间（100纳秒为单位）
    pub duration: i64,        // 持续时间（100纳秒为单位，-1表示未知）
    pub description: String,  // UTF-8描述
}

#[derive(Debug)]
pub struct EdfHeader {
    pub file_type: FileType,
    pub signals: Vec<SignalParam>,
    pub file_duration: i64,           // 文件持续时间（100纳秒为单位）
    pub start_date: NaiveDate,
    pub start_time: NaiveTime,
    pub starttime_subsecond: i64,     // 亚秒开始时间
    pub datarecords_in_file: i64,
    pub datarecord_duration: i64,     // 数据记录持续时间（100纳秒为单位）
    pub annotations_in_file: i64,
    
    // EDF+ 特有字段
    pub patient_code: String,
    pub sex: String,
    pub birthdate: String,
    pub patient_name: String,
    pub patient_additional: String,
    pub admin_code: String,
    pub technician: String,
    pub equipment: String,
    pub recording_additional: String,
}
