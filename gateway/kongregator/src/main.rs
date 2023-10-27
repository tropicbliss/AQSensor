use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use lazy_static::lazy_static;
use prometheus::{
    labels, opts, register_gauge, register_int_gauge, Encoder, Gauge, IntGauge, TextEncoder,
};
use reqwest::{Client, ClientBuilder, Url};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

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
    input: AirQualityInput,
    client: Client,
    alarm_url: Url,
    minimum_co2_ppm: u64,
}

impl Metrics {
    fn new() -> Result<Self> {
        Ok(Self {
            input: Default::default(),
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
        .route("/jsonmetrics", get(metrics_json))
        .route("/adjustppm/:min", get(change_minimum_co2_ppm))
        .with_state(state);
    let port = std::env::var("PORT")?;
    let addr = SocketAddr::from(([0, 0, 0, 0], port.parse()?));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

fn calculate_heat_index(temp: f64, hum: f64) -> f64 {
    let heat_index = -8.784_694_755_56
        + (1.611_394_11 * temp)
        + (2.338_548_838_89 * hum)
        + (-0.146_116_05 * temp * hum)
        + (-0.012_308_094 * temp.powi(2))
        + (-0.016_424_827_7778 * hum.powi(2))
        + (2.211_732 * 1e-3 * temp.powi(2) * hum)
        + (7.2546 * 1e-4 * temp * hum.powi(2))
        + (-3.582 * 1e-6 * temp.powi(2) * hum.powi(2));
    heat_index
}

async fn export_metrics(State(metrics): State<Arc<Mutex<Metrics>>>) -> impl IntoResponse {
    {
        let metrics = metrics.lock().unwrap().input.clone();
        PM02_GAUGE.set(metrics.pm25);
        RCO2_GAUGE.set(metrics.co2);
        ATMP_GAUGE.set(metrics.temp);
        RHUM_GAUGE.set(metrics.hum);
        let heat_index = calculate_heat_index(metrics.temp as f64, metrics.hum as f64);
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

async fn metrics_json(State(metrics): State<Arc<Mutex<Metrics>>>) -> impl IntoResponse {
    let metrics = metrics.lock().unwrap().input.clone();
    let heat_index = calculate_heat_index(metrics.temp as f64, metrics.hum as f64);
    let output = AirQualityOutput {
        co2: metrics.co2,
        heat_index,
        hum: metrics.hum,
        pm25: metrics.pm25,
        temp: metrics.temp,
        wifi: metrics.wifi,
    };
    Json(output)
}

async fn change_minimum_co2_ppm(
    State(metrics): State<Arc<Mutex<Metrics>>>,
    Path(min_co2): Path<u64>,
) -> impl IntoResponse {
    let mut metrics = metrics.lock().unwrap();
    metrics.minimum_co2_ppm = min_co2;
    tracing::debug!("set min ppm to {min_co2}");
    StatusCode::OK
}

#[derive(Serialize)]
struct AirQualityOutput {
    wifi: i64,
    co2: i64,
    pm25: i64,
    temp: f64,
    hum: i64,
    #[serde(rename(serialize = "heatIndex"))]
    heat_index: f64,
}

#[derive(Deserialize, Debug, Default, Clone)]
struct AirQualityInput {
    wifi: i64,
    #[serde(alias = "rco2")]
    co2: i64,
    #[serde(alias = "pm02")]
    pm25: i64,
    #[serde(alias = "atmp")]
    temp: f64,
    #[serde(alias = "rhum")]
    hum: i64,
}

async fn air_quality_input(
    State(metrics): State<Arc<Mutex<Metrics>>>,
    Json(payload): Json<AirQualityInput>,
) -> impl IntoResponse {
    tracing::debug!("{:?}", payload);
    let mut metrics = metrics.lock().unwrap();
    metrics.input = payload;
    if metrics.input.co2 as u64 >= metrics.minimum_co2_ppm {
        let client = metrics.client.clone();
        let alarm_url = metrics.alarm_url.clone();
        tokio::spawn(async move {
            client.get(alarm_url).send().await.unwrap();
        });
    }
    StatusCode::CREATED
}
