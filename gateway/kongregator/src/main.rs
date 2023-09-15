use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use lazy_static::lazy_static;
use prometheus::{
    labels, opts, register_gauge, register_int_gauge, Encoder, Gauge, IntGauge, TextEncoder,
};
use reqwest::{Client, ClientBuilder};
use serde::Deserialize;
use std::{
    fmt::Debug,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use url::Url;

lazy_static! {
    static ref PM02_GAUGE: IntGauge = register_int_gauge!(opts!(
        "pm02",
        "Particulate Matter PM2.5 value",
        labels! {"id" => "Airgradient"}
    ))
    .unwrap();
    static ref RCO2_GAUGE: IntGauge = register_int_gauge!(opts!(
        "rco2",
        "CO2 value, in ppm",
        labels! {"id" => "Airgradient"}
    ))
    .unwrap();
    static ref ATMP_GAUGE: Gauge = register_gauge!(opts!(
        "atmp",
        "Temperature, in degrees Celcius",
        labels! {"id" => "Airgradient"}
    ))
    .unwrap();
    static ref RHUM_GAUGE: IntGauge = register_int_gauge!(opts!(
        "rhum",
        "Relative humidity, in percent",
        labels! {"id" => "Airgradient"}
    ))
    .unwrap();
    static ref HEAT_INDEX: Gauge = register_gauge!(opts!(
        "heat",
        "Heat index, in degrees Celcius",
        labels! {"id" => "Extra"}
    ))
    .unwrap();
    static ref WIFI: IntGauge = register_int_gauge!(opts!(
        "wifi",
        "WiFi RSSI of Airgradient",
        labels! {"id" => "Extra"}
    ))
    .unwrap();
}

struct Metrics {
    co2: i64,
    pm25: i64,
    temp: f64,
    hum: i64,
    wifi: i64,
    client: Client,
    alarm_url: Url,
    minimum_co2_ppm: u64,
}

impl Metrics {
    fn new() -> Result<Self> {
        Ok(Self {
            co2: 0,
            pm25: 0,
            temp: 0.0,
            hum: 0,
            wifi: 0,
            client: ClientBuilder::new()
                .timeout(Duration::from_secs(3))
                .build()?,
            alarm_url: Url::parse(&std::env::var("ALARM_URL")?)?,
            minimum_co2_ppm: std::env::var("MIN_CO2_PPM")?.parse()?,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kongregator=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    let state = Arc::new(Mutex::new(Metrics::new()?));
    let app = Router::new()
        .route("/input", post(air_quality_input))
        .route("/metrics", get(export_metrics))
        .with_state(state);
    let port = std::env::var("PORT")?;
    let addr = SocketAddr::from(([0, 0, 0, 0], port.parse()?));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn export_metrics(State(metrics): State<Arc<Mutex<Metrics>>>) -> impl IntoResponse {
    {
        let metrics = metrics.lock().unwrap();
        PM02_GAUGE.set(metrics.pm25);
        RCO2_GAUGE.set(metrics.co2);
        ATMP_GAUGE.set(metrics.temp);
        RHUM_GAUGE.set(metrics.hum);
        let heat_index = -8.784_694_755_56
            + (1.611_394_11 * metrics.temp as f64)
            + (2.338_548_838_89 * metrics.hum as f64)
            + (-0.146_116_05 * metrics.temp as f64 * metrics.hum as f64)
            + (-0.012_308_094 * (metrics.temp as f64).powi(2))
            + (-0.016_424_827_7778 * (metrics.hum as f64).powi(2))
            + (2.211_732 * 1e-3 * (metrics.temp as f64).powi(2) * metrics.hum as f64)
            + (7.2546 * 1e-4 * metrics.temp as f64 * (metrics.hum as f64).powi(2))
            + (-3.582 * 1e-6 * (metrics.temp as f64).powi(2) * (metrics.hum as f64).powi(2));
        HEAT_INDEX.set(heat_index);
        WIFI.set(metrics.wifi);
    }
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    output
}

#[derive(Deserialize, Debug)]
struct AirQualityInput {
    wifi: i64,
    #[serde(rename = "rco2")]
    co2: i64,
    #[serde(rename = "pm02")]
    pm25: i64,
    #[serde(rename = "atmp")]
    temp: f64,
    #[serde(rename = "rhum")]
    hum: i64,
}

async fn air_quality_input(
    State(metrics): State<Arc<Mutex<Metrics>>>,
    Json(payload): Json<AirQualityInput>,
) -> impl IntoResponse {
    tracing::debug!("{:?}", payload);
    let mut metrics = metrics.lock().unwrap();
    metrics.co2 = payload.co2;
    metrics.hum = payload.hum;
    metrics.pm25 = payload.pm25;
    metrics.temp = payload.temp;
    metrics.wifi = payload.wifi;
    if payload.co2 as u64 >= metrics.minimum_co2_ppm {
        let client = metrics.client.clone();
        let alarm_url = metrics.alarm_url.clone();
        tokio::spawn(async move {
            client.get(alarm_url).send().await.unwrap();
        });
    }
    (StatusCode::CREATED, "")
}
