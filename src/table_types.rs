use tabled::Tabled;

include!("hwinfo_types.rs");

#[derive(Tabled)]
struct Sensor {
    id: u32,
    instance: u32,
    name: &'static str,
}

#[derive(Tabled)]
struct Reading {
    id: u32,
    sensor_index: u32,
    label: &'static str,
    unit: &'static str,
    value: f64,
    min_value: f64,
    avg_value: f64,
    max_value: f64,
    reading_type: SensorReadingType,
}