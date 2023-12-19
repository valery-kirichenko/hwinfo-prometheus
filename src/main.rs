#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::{Duration, SystemTime};

use axum::{Extension, Router};
use axum::routing::get;
use directories::ProjectDirs;
use log::{error, info, warn};
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;
use tokio::runtime::Handle;
use tokio::sync::{mpsc, RwLock};
use tokio::sync::mpsc::{Receiver, Sender};

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
    tx_rq: Sender<()>,
    rx_rs: Receiver<()>,
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
    setup_logger();

    let metrics = Arc::new(RwLock::new(Metrics { ..Default::default() }));
    let (tx_rq, mut rx_rq) = mpsc::channel::<()>(1);
    let (tx_rs, rx_rs) = mpsc::channel::<()>(1);
    let shared_state = Arc::new(RwLock::new(AppState {
        registry: <Registry>::default(),
        tx_rq,
        rx_rs,
    }));

    let mut state = shared_state.write().await;
    let metrics_read = metrics.read().await;
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
    info!("Listening on {}", listener.local_addr().unwrap());

    let handle = Handle::current();
    std::thread::spawn(move || {
        let mut reader: Reader;
        let mut attempts = 0;
        loop {
            match Reader::new() {
                Ok(reader_instance) => {
                    reader = reader_instance;
                    info!("HWiNFO Reader is ready");
                    break;
                },
                Err(_) => {
                    if attempts > 5 {
                        error!("HWiNFO is not available, retries exceeded. Exiting...");
                        std::process::exit(1);
                    }
                    warn!("HWiNFO is not available, retrying in 5s");
                    attempts += 1;
                    std::thread::sleep(Duration::from_secs(5));
                },
            };
        }

        loop {
            handle.block_on(async {
                rx_rq.recv().await;
            });
            reader.update_readings();
            for reading in &reader.readings {
                let sensor = reader.sensors.get(reading.sensor_index as usize).unwrap();
                let reading_type = reading.reading_type;
                handle.block_on(async {
                    metrics.read().await.gauge_reading(HWiNFOLabels
                                                       {
                                                           sensor: utf8_to_str(&sensor.user_name_utf8),
                                                           reading: utf8_to_str(&reading.user_label_utf8),
                                                           unit: utf8_to_str(&reading.unit_utf8),
                                                       }, reading_type, reading.value);
                });
            }
            handle.block_on(async {
                tx_rs.send(()).await.expect("Unable to send a response");
            });
        }
    });

    axum::serve(listener, app).await.unwrap();
}

async fn handler(Extension(state): Extension<SharedState>) -> String {
    let mut state = state.write().await;
    state.tx_rq.send(()).await.expect("Unable to send a request");
    state.rx_rs.recv().await;

    let mut body = String::new();
    encode(&mut body, &state.registry).unwrap();

    body
}

fn setup_logger() {
    let dirs = ProjectDirs::from("dev", "Valery Kirichenko", "HWiNFO Prometheus").unwrap();
    let log_path = dirs.data_local_dir().join("output.log");
    std::fs::create_dir_all(dirs.data_local_dir()).unwrap();
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {:5}] {}",
                humantime::format_rfc3339_millis(SystemTime::now()),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file(log_path).unwrap())
        .apply().unwrap();
}
