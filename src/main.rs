// #![windows_subsystem = "windows"]

use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicU64;
use std::thread;
use std::time::Duration;

use axum::{Extension, Router};
use axum::routing::get;
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;

use crate::hwinfo_reader::Reader;
use crate::hwinfo_types::SensorReadingType;

mod hwinfo_types;
mod hwinfo_reader;
mod table_types;

fn utf8_to_str(utf8: &[u8]) -> String {
    std::ffi::CStr::from_bytes_until_nul(utf8).unwrap().to_str().unwrap().to_string()
}

struct AppState {
    registry: Registry,
}

type SharedState = Arc<RwLock<AppState>>;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct HWiNFOLabels {
    pub sensor: String,
    pub reading: String,
    pub unit: String,
}

#[derive(Default)]
pub struct Metrics {
    temperature: Family<HWiNFOLabels, Gauge<f64, AtomicU64>>,
    voltage: Family<HWiNFOLabels, Gauge<f64, AtomicU64>>,
    fan_speed: Family<HWiNFOLabels, Gauge<f64, AtomicU64>>,
    current: Family<HWiNFOLabels, Gauge<f64, AtomicU64>>,
    power: Family<HWiNFOLabels, Gauge<f64, AtomicU64>>,
    clock: Family<HWiNFOLabels, Gauge<f64, AtomicU64>>,
    usage: Family<HWiNFOLabels, Gauge<f64, AtomicU64>>,
    other: Family<HWiNFOLabels, Gauge<f64, AtomicU64>>,
}

impl Metrics {
    pub fn gauge_reading(&self, label: HWiNFOLabels, reading_type: SensorReadingType, value: f64) {
        match reading_type {
            SensorReadingType::None => {}
            SensorReadingType::Temp => { self.temperature.get_or_create(&label).set(value); }
            SensorReadingType::Volt => { self.voltage.get_or_create(&label).set(value); }
            SensorReadingType::Fan => { self.fan_speed.get_or_create(&label).set(value); }
            SensorReadingType::Current => { self.current.get_or_create(&label).set(value); }
            SensorReadingType::Power => { self.power.get_or_create(&label).set(value); }
            SensorReadingType::Clock => { self.clock.get_or_create(&label).set(value); }
            SensorReadingType::Usage => { self.usage.get_or_create(&label).set(value); }
            SensorReadingType::Other => { self.other.get_or_create(&label).set(value); }
        };
    }
}

#[tokio::main]
async fn main() {
    let metrics = Arc::new(RwLock::new(Metrics { ..Default::default() }));
    let shared_state = Arc::new(RwLock::new(AppState {
        registry: <Registry>::default()
    }));

    let mut state = shared_state.write().unwrap();
    let metrics_read = metrics.read().unwrap();
    state.registry.register("temperature", "Temperature measurement", metrics_read.temperature.clone());
    state.registry.register("voltage", "Voltage measurement", metrics_read.voltage.clone());
    state.registry.register("fan_speed", "Fan speed measurement", metrics_read.fan_speed.clone());
    state.registry.register("current", "Current measurement", metrics_read.current.clone());
    state.registry.register("power", "Power measurement", metrics_read.power.clone());
    state.registry.register("clock", "Clock speed measurement", metrics_read.clock.clone());
    state.registry.register("usage", "Usage measurement", metrics_read.usage.clone());
    state.registry.register("other", "Arbitrary value with its own unit", metrics_read.other.clone());
    drop(state);
    drop(metrics_read);

    let app = Router::new()
        .route("/metrics", get(handler))
        .layer(Extension(shared_state))
        .layer(Extension(metrics.clone()));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());

    thread::spawn(move || {
        let mut reader = match Reader::new() {
            Ok(reader) => reader,
            Err(e) => panic!("{}", e.to_string()),
        };
        let polling_period = reader.info.polling_period;
        println!("Will refresh readings every {} ms", polling_period);

        loop {
            for reading in &reader.readings {
                let sensor = reader.sensors.get(reading.sensor_index as usize).unwrap();
                let reading_type = reading.reading_type;
                metrics.read().unwrap().gauge_reading(
                    HWiNFOLabels
                    {
                        sensor: utf8_to_str(&sensor.user_name_utf8),
                        reading: utf8_to_str(&reading.user_label_utf8),
                        unit: utf8_to_str(&reading.unit_utf8),
                    }, reading_type, reading.value);
            }
            thread::sleep(Duration::from_millis(reader.info.polling_period as u64));
            reader.update_readings();
        }
    });

    axum::serve(listener, app).await.unwrap();
}

async fn handler(Extension(state): Extension<SharedState>) -> String {
    let state = state.read().unwrap();

    let mut body = String::new();
    encode(&mut body, &state.registry).unwrap();

    body
}