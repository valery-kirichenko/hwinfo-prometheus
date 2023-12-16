use std::fmt::{Display, Formatter};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
#[allow(unused)]
enum SensorReadingType {
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
#[derive(Debug)]
struct HWiNFOReadingElement {
    reading_type: SensorReadingType,
    sensor_index: u32,
    reading_id: u32,
    original_label: [i8;128],
    user_label: [i8;128],
    unit: [i8;16],
    value: f64,
    min_value: f64,
    max_value: f64,
    avg_value: f64,
    user_label_utf8: [u8;128],
    unit_utf8: [u8;16],
}

#[repr(C, packed(1))]
#[derive(Debug)]
struct HWiNFOSensorElement {
    sensor_id: u32,
    sensor_instance: u32,
    original_name: [i8;128],
    user_name: [i8;128],
    user_name_utf8: [u8;128],
}

#[repr(C, packed(1))]
#[derive(Debug)]
struct HWiNFOSharedMemory {
    signature: u32,
    version: u32,
    revision: u32,
    poll_time: i64,
    sensor_section_offset: u32,
    sensor_element_size: u32,
    sensor_elements_number: u32,
    reading_section_offset: u32,
    reading_element_size: u32,
    reading_elements_number: u32,
    polling_period: u32,
}
