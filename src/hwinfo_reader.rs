use std::error::Error;
use std::fmt::{Display, Formatter};
use log::info;

use tabled::settings::{Alignment, Modify, Panel, Style};
use tabled::settings::object::Rows;
use tabled::Table;
use win_sys::{FILE_MAP_READ, ViewOfFile};

use crate::hwinfo_types::{HWiNFOReadingElement, HWiNFOSensorElement, HWiNFOSharedMemory};
use crate::table_types::Reading;
use crate::utf8_to_str;

pub struct Reader<'a> {
    pub info: &'a HWiNFOSharedMemory,
    pub sensors: Vec<&'a HWiNFOSensorElement>,
    pub readings: Vec<&'a HWiNFOReadingElement>,

    _view: ViewOfFile,
    svm_ptr: *const HWiNFOSharedMemory,
    readings_ptr: *const HWiNFOReadingElement,
}

impl Reader<'_> {
    pub fn new() -> Result<Self, SVMOpenError> {
        let file_mapping = match win_sys::OpenFileMapping(FILE_MAP_READ, false, "Global\\HWiNFO_SENS_SM2") {
            Ok(mapping) => mapping,
            Err(_) => return Err(SVMOpenError),
        };
        let _view = win_sys::MapViewOfFile(file_mapping.as_handle(), FILE_MAP_READ, 0, 0, 0).unwrap();
        let svm_ptr = _view.as_mut_ptr() as *const HWiNFOSharedMemory;
        let info = unsafe { &(*svm_ptr) };
        let mut sensors_ptr = unsafe { svm_ptr.add(1) as *const HWiNFOSensorElement };

        let mut sensors = Vec::with_capacity(info.sensor_elements_number as usize);
        for _ in 0..info.sensor_elements_number {
            let element = unsafe { &(*sensors_ptr) };
            sensors.push(element);
            sensors_ptr = unsafe { sensors_ptr.add(1) };
        }

        let mut result = Self {
            info,
            sensors,
            readings: Vec::with_capacity(info.reading_elements_number as usize),
            _view,
            svm_ptr,
            readings_ptr: sensors_ptr as *const HWiNFOReadingElement,
        };
        result.update_readings();

        Ok(result)
    }

    pub fn update_readings(&mut self) {
        self.info = unsafe { &(*self.svm_ptr) };

        let mut readings_ptr = self.readings_ptr;
        self.readings.clear();
        for _ in 0..self.info.reading_elements_number {
            let reading = unsafe { &(*readings_ptr) };
            self.readings.push(reading);
            readings_ptr = unsafe { readings_ptr.add(1) };
        }
    }

    #[allow(unused)]
    pub fn print_readings(&self) {
        let mut readings = Vec::with_capacity(self.readings.len());
        for reading in &self.readings {
            readings.push(Reading {
                id: reading.reading_id,
                sensor_name: utf8_to_str(&self.sensors.get(reading.sensor_index as usize).unwrap().user_name_utf8),
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
        info!("{}", table);
    }
}

#[derive(Debug, Clone)]
pub struct SVMOpenError;

impl Display for SVMOpenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Can't read shared memory. Is HWiNFO running and shared memory support is enabled?")
    }
}

impl Error for SVMOpenError {}
