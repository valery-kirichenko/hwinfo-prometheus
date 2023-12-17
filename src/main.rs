use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicU64;
use std::thread;
use std::time::Duration;

use axum::{Extension, Router};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::routing::get;
use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};
use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;

use crate::hwinfo_reader::Reader;
use crate::hwinfo_types::SensorReadingType;

mod hwinfo_types;
mod hwinfo_reader;

fn utf8_to_str(utf8: &[u8]) -> String {
    std::ffi::CStr::from_bytes_until_nul(utf8).unwrap().to_str().unwrap().to_string()
}

struct AppState {
    registry: Registry,
}

type SharedState = Arc<RwLock<AppState>>;
type SharedMetrics = Arc<RwLock<Metrics>>;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct HWiNFOLabels {
    pub sensor: String,
    pub reading: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct HWiNFOExtendedLabels {
    pub sensor: String,
    pub reading: String,
    pub unit: String,
}

pub struct Metrics {
    temperature: Family<HWiNFOLabels, Gauge<f64, AtomicU64>>,
    voltage: Family<HWiNFOLabels, Gauge<f64, AtomicU64>>,
    fan_speed: Family<HWiNFOLabels, Gauge<f64, AtomicU64>>,
    current: Family<HWiNFOLabels, Gauge<f64, AtomicU64>>,
    power: Family<HWiNFOLabels, Gauge<f64, AtomicU64>>,
    clock: Family<HWiNFOLabels, Gauge<f64, AtomicU64>>,
    usage: Family<HWiNFOLabels, Gauge<f64, AtomicU64>>,
    other: Family<HWiNFOExtendedLabels, Gauge<f64, AtomicU64>>,
}

impl Metrics {
    pub fn gauge_general_reading(&self, label: HWiNFOLabels, reading_type: SensorReadingType, value: f64) {
        match reading_type {
            SensorReadingType::None => {}
            SensorReadingType::Temp => { self.temperature.get_or_create(&label).set(value); }
            SensorReadingType::Volt => { self.voltage.get_or_create(&label).set(value); }
            SensorReadingType::Fan => { self.fan_speed.get_or_create(&label).set(value); }
            SensorReadingType::Current => { self.current.get_or_create(&label).set(value); }
            SensorReadingType::Power => { self.power.get_or_create(&label).set(value); }
            SensorReadingType::Clock => { self.clock.get_or_create(&label).set(value); }
            SensorReadingType::Usage => { self.usage.get_or_create(&label).set(value); }
            SensorReadingType::Other => {}
        };
    }

    pub fn gauge_other_reading(&self, label: HWiNFOExtendedLabels, value: f64) {
        self.other.get_or_create(&label).set(value);
    }
}

#[tokio::main]
async fn main() {
    let metrics = Arc::new(RwLock::new(Metrics {
        temperature: Family::default(),
        voltage: Family::default(),
        fan_speed: Family::default(),
        current: Family::default(),
        power: Family::default(),
        clock: Family::default(),
        usage: Family::default(),
        other: Family::default(),
    }));
    let shared_state = Arc::new(RwLock::new(AppState {
        registry: <Registry>::default()
    }));

    {
        let mut state = shared_state.write().unwrap();
        let metrics = metrics.read().unwrap();
        state.registry.register("temperature", "Temperature in degrees Celsius", metrics.temperature.clone());
        state.registry.register("voltage", "Voltage in Volts", metrics.voltage.clone());
        state.registry.register("fan_speed", "Fan speed in RPM", metrics.fan_speed.clone());
        state.registry.register("current", "Current in Amps", metrics.current.clone());
        state.registry.register("power", "Power in watts", metrics.power.clone());
        state.registry.register("clock", "Clock speed in MHz", metrics.clock.clone());
        state.registry.register("usage", "Usage in percents", metrics.usage.clone());
        state.registry.register("other", "Arbitrary value with its own unit", metrics.other.clone());
    }

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
                if reading_type != SensorReadingType::Other {
                    metrics.read().unwrap().gauge_general_reading(
                        HWiNFOLabels
                        {
                            sensor: utf8_to_str(&sensor.user_name_utf8),
                            reading: utf8_to_str(&reading.user_label_utf8),
                        }, reading_type, reading.value);
                } else {
                    metrics.read().unwrap().gauge_other_reading(
                        HWiNFOExtendedLabels
                        {
                            sensor: utf8_to_str(&sensor.user_name_utf8),
                            reading: utf8_to_str(&reading.user_label_utf8),
                            unit: utf8_to_str(&reading.unit_utf8),
                        }, reading.value);
                }
            }
            thread::sleep(Duration::from_millis(reader.info.polling_period as u64));
            reader.update_readings();
        }
    });

    axum::serve(listener, app).await.unwrap();
}

async fn handler(Extension(state): Extension<SharedState>, Extension(metrics): Extension<SharedMetrics>) -> impl IntoResponse {
    let state = state.read().unwrap();

    let mut body = String::new();
    encode(&mut body, &state.registry).unwrap();

    let mut headers = HeaderMap::new();
    // headers.insert(CONTENT_TYPE, "application/openmetrics-text; version=1.0.0; charset=utf-8".parse().unwrap());
    (headers, body)
}