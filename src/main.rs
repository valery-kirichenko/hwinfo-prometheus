use std::time::Duration;

use tabled::settings::{Alignment, Modify, Panel, Style};
use tabled::settings::object::Rows;
use tabled::Table;
use crate::hwinfo_reader::Reader;

use crate::table_types::Reading;

mod hwinfo_types;
mod table_types;
mod hwinfo_reader;

fn utf8_to_str(utf8: &[u8]) -> String {
    std::ffi::CStr::from_bytes_until_nul(utf8).unwrap().to_str().unwrap().to_string()
}

fn main() -> Result<(), String> {
    let mut reader = match Reader::new() {
        Ok(reader) => reader,
        Err(e) => return Err(e.to_string()),
    };

    loop {
        print_readings(&reader);
        std::thread::sleep(Duration::from_millis(reader.info.polling_period as u64));
        reader.update_readings();
    }
}

fn print_readings(reader: &Reader) {
    let mut readings = Vec::with_capacity(reader.readings.len());
    for reading in &reader.readings {
        readings.push(Reading {
            id: reading.reading_id,
            sensor_name: utf8_to_str(&reader.sensors.get(reading.sensor_index as usize).unwrap().user_name_utf8),
            label: utf8_to_str(&reading.user_label_utf8),
            unit: utf8_to_str(&reading.unit_utf8),
            value: reading.value,
            min_value: reading.min_value,
            avg_value: reading.avg_value,
            max_value: reading.max_value,
            reading_type: reading.reading_type,
        });
    }
    let table = Table::builder(&readings)
        .index()
        .build()
        .with(Panel::header("Readings"))
        .with(Style::sharp())
        .with(Modify::new(Rows::first()).with(Alignment::center()))
        .to_string();
    println!("{}", table);
}
