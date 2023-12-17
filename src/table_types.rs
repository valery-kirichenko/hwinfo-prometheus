use tabled::Tabled;
use crate::hwinfo_types::SensorReadingType;

#[derive(Tabled)]
pub struct Sensor {
    pub id: u32,
    pub instance: u32,
    pub name: &'static str,
}

#[derive(Tabled)]
pub struct Reading {
    pub id: u32,
    pub sensor_name: String,
    pub label: String,
    pub unit: String,
    pub value: f64,
    pub min_value: f64,
    pub avg_value: f64,
    pub max_value: f64,
    pub reading_type: SensorReadingType,
}