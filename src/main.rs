mod hwinfo_types;
mod table_types;

use std::error::Error;
use tabled::settings::{Alignment, Modify, Panel, Style};
use tabled::settings::object::Rows;
use tabled::Table;
use win_sys::FILE_MAP_READ;
use crate::hwinfo_types::{HWiNFOReadingElement, HWiNFOSensorElement, HWiNFOSharedMemory};
use crate::table_types::{Reading, Sensor};

fn utf8_to_str(utf8: &[u8]) -> &str {
    std::ffi::CStr::from_bytes_until_nul(utf8).unwrap().to_str().unwrap()
}

fn main() -> Result<(), String> {
    let file_mapping = match win_sys::OpenFileMapping(FILE_MAP_READ, false, "Global\\HWiNFO_SENS_SM2") {
        Ok(mapping) => mapping,
        Err(_) => return Err("Can't read shared memory. Is HWiNFO running and shared memory support is enabled?".to_string()),
    };
    let view = win_sys::MapViewOfFile(file_mapping.as_handle(), FILE_MAP_READ, 0, 0, 0).unwrap();
    let svm_ptr = view.as_mut_ptr() as *const HWiNFOSharedMemory;
    let info = unsafe { &(*svm_ptr) };
    println!("{:?}", info);
    let mut sensors_ptr = unsafe { svm_ptr.add(1) as *const HWiNFOSensorElement };

    let mut sensors = Vec::with_capacity(info.sensor_elements_number as usize);
    for _ in 0..info.sensor_elements_number {
        let element = unsafe { &(*sensors_ptr) };
        sensors.push(Sensor {
            id: element.sensor_id,
            instance: element.sensor_instance,
            name: utf8_to_str(&element.user_name_utf8),
        });
        sensors_ptr = unsafe { sensors_ptr.add(1) };
    }
    let table = Table::builder(&sensors)
        .index()
        .build()
        .with(Panel::header("Sensors"))
        .with(Style::modern())
        .with(Modify::new(Rows::first()).with(Alignment::center()))
        .to_string();
    println!("{}", table);

    print_readings(sensors_ptr as *const HWiNFOReadingElement, info.reading_elements_number as usize);
    Ok(())
}

fn print_readings(mut reading_ptr: *const HWiNFOReadingElement, reading_elements_number: usize) {
    let mut readings = Vec::with_capacity(reading_elements_number);
    for _ in 0..reading_elements_number {
        let reading = unsafe { &(*reading_ptr) };
        readings.push(Reading {
            id: reading.reading_id,
            sensor_index: reading.sensor_index,
            label: utf8_to_str(&reading.user_label_utf8),
            unit: utf8_to_str(&reading.unit_utf8),
            value: reading.value,
            min_value: reading.min_value,
            avg_value: reading.avg_value,
            max_value: reading.max_value,
            reading_type: reading.reading_type,
        });
        reading_ptr = unsafe { reading_ptr.add(1) };
    }
    let table = Table::new(readings)
        .with(Panel::header("Readings"))
        .with(Style::modern())
        .with(Modify::new(Rows::first()).with(Alignment::center()))
        .to_string();
    println!("{}", table);
}