use std::fmt::{Display, Formatter};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
#[allow(unused)]
pub enum SensorReadingType {
    None,
    Temp,
    Volt,
    Fan,
    Current,
    Power,
    Clock,
    Usage,
    Other
}

impl Display for SensorReadingType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[repr(C, packed(1))]
#[derive(Debug, Copy, Clone)]
pub struct HWiNFOReadingElement {
    pub reading_type: SensorReadingType,
    pub sensor_index: u32,
    pub reading_id: u32,
    pub original_label: [i8;128],
    pub user_label: [i8;128],
    pub unit: [i8;16],
    pub value: f64,
    pub min_value: f64,
    pub max_value: f64,
    pub avg_value: f64,
    pub user_label_utf8: [u8;128],
    pub unit_utf8: [u8;16],
}

#[repr(C, packed(1))]
#[derive(Debug, Copy, Clone)]
pub struct HWiNFOSensorElement {
    pub sensor_id: u32,
    pub sensor_instance: u32,
    pub original_name: [i8;128],
    pub user_name: [i8;128],
    pub user_name_utf8: [u8;128],
}

#[repr(C, packed(1))]
#[derive(Debug, Copy, Clone)]
pub struct HWiNFOSharedMemory {
    pub signature: u32,
    pub version: u32,
    pub revision: u32,
    pub poll_time: i64,
    pub sensor_section_offset: u32,
    pub sensor_element_size: u32,
    pub sensor_elements_number: u32,
    pub reading_section_offset: u32,
    pub reading_element_size: u32,
    pub reading_elements_number: u32,
    pub polling_period: u32,
}
