use super::config::*;
use opentelemetry::{
    api,
    api::{context::propagation::text_propagator::HttpTextFormat, Provider},
    sdk,
};
use std::collections::HashMap;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use http::{header::HeaderName, HeaderMap};
use tonic::metadata::AsciiMetadataValue;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn get_tracing_header() -> Option<(&'static str, String)> {
    let span = Span::current();
    let span_ctx = span.context();
    let propagator = api::TraceContextPropagator::new();
    let mut carrier = std::collections::HashMap::new();
    propagator.inject_context(&span_ctx, &mut carrier);
    carrier.get("traceparent").map(|f| ("traceparent", f.to_string()))
}

pub fn get_grpc_tracing_header() -> Option<(&'static str, AsciiMetadataValue)> {
    get_tracing_header().map(|f| (f.0, AsciiMetadataValue::from_str(&f.1).unwrap()))
}

pub fn from_http_headers(span: Span, header_map: &HeaderMap) -> Span {
    let trace_header = HeaderName::from_lowercase("traceparent".as_bytes()).unwrap();

    if let Some(value) = header_map.get(trace_header) {
        let propagator = api::TraceContextPropagator::new();
        let mut carrier = std::collections::HashMap::new();
        carrier.insert("traceparent".to_string(), String::from_utf8_lossy(value.as_bytes()).to_string());
        let ctx = propagator.extract(&carrier);
        span.set_parent(&ctx);
        span
    } else {
        span
    }
}

pub fn from_bytes(span: Span, value: &[u8]) -> Span {
    let propagator = api::TraceContextPropagator::new();
    let mut carrier = std::collections::HashMap::new();
    carrier.insert("traceparent".to_string(), String::from_utf8_lossy(value).to_string());
    let ctx = propagator.extract(&carrier);
    span.set_parent(&ctx);
    span
}

pub fn from_map(span: Span, map: &HashMap<String, String>) -> Span {
    let propagator = api::TraceContextPropagator::new();
    let ctx = propagator.extract(map);
    span.set_parent(&ctx);
    span
}

pub fn init_tracer(config: &Swir) -> Result<(), Box<dyn std::error::Error>> {
    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info")).unwrap();
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).finish();
    let registry = tracing_subscriber::registry().with(filter_layer).with(fmt_layer);

    if let Some(cfg) = &config.tracing {
        if let Some(open_telemetry) = &cfg.open_telemetry {
            debug!("Open telementry tracing selected {:?}", open_telemetry);
            let exporter = opentelemetry_jaeger::Exporter::builder()
                .with_agent_endpoint(format!("{}:{}", open_telemetry.collector_address, open_telemetry.collector_port).parse().unwrap())
                .with_process(opentelemetry_jaeger::Process {
                    service_name: open_telemetry.service_name.clone(),
                    tags: Vec::new(),
                })
                .init()?;
            let provider = sdk::Provider::builder()
                .with_simple_exporter(exporter)
                .with_config(sdk::Config {
                    default_sampler: Box::new(sdk::Sampler::AlwaysOn),
                    ..Default::default()
                })
                .build();
            let tracer = provider.get_tracer("tracing");
            let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);
            let registry = registry.with(opentelemetry);
            if let Err(e) = registry.try_init() {
                println!("Unable to initialise the tracer {}", e);
                Err(Box::new(e))
            } else {
                info!("Using  opentelemetry tracer");
                Ok(())
            }
        } else {
            Ok(registry.try_init()?)
        }
    } else {
        Ok(registry.try_init()?)
    }
}

#[allow(dead_code)]
fn init_tracer_no_conf() -> Result<(), Box<dyn std::error::Error>> {
    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info")).unwrap();
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).finish();
    let registry = tracing_subscriber::registry().with(filter_layer).with(fmt_layer);

    Ok(registry.try_init()?)
}
