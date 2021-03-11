use super::config::*;
use opentelemetry::global;
use opentelemetry::propagation::TextMapPropagator;
use opentelemetry::sdk::{
    propagation::TraceContextPropagator,
    trace,
    trace::{IdGenerator, Sampler},
    Resource,
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
    let tc_propagator = TraceContextPropagator::new();
    let mut carrier = std::collections::HashMap::new();
    tc_propagator.inject_context(&span_ctx, &mut carrier);
    carrier.get("traceparent").map(|f| ("traceparent", f.to_string()))
}

pub fn get_grpc_tracing_header() -> Option<(&'static str, AsciiMetadataValue)> {
    get_tracing_header().map(|f| (f.0, AsciiMetadataValue::from_str(&f.1).unwrap()))
}

pub fn from_http_headers(span: Span, header_map: &HeaderMap) -> Span {
    let trace_header = HeaderName::from_lowercase("traceparent".as_bytes()).unwrap();

    if let Some(value) = header_map.get(trace_header) {
        let propagator = TraceContextPropagator::new();
        let mut carrier = std::collections::HashMap::new();
        carrier.insert("traceparent".to_string(), String::from_utf8_lossy(value.as_bytes()).to_string());
        let ctx = propagator.extract(&carrier);
        span.set_parent(ctx);
        span
    } else {
        span
    }
}

pub fn from_bytes(span: Span, value: &[u8]) -> Span {
    let propagator = TraceContextPropagator::new();
    let mut carrier = std::collections::HashMap::new();
    carrier.insert("traceparent".to_string(), String::from_utf8_lossy(value).to_string());
    let ctx = propagator.extract(&carrier);
    span.set_parent(ctx);
    span
}

pub fn from_map(span: Span, map: &HashMap<String, String>) -> Span {
    let propagator = TraceContextPropagator::new();
    let ctx = propagator.extract(map);
    span.set_parent(ctx);
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

            global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());
            let (tracer, _uninstall) = opentelemetry_jaeger::new_pipeline()
                .from_env()
                .with_agent_endpoint(format!("{}:{}", open_telemetry.collector_address, open_telemetry.collector_port))
                .with_service_name(&open_telemetry.service_name)
                .with_tags(vec![])
                .with_trace_config(
                    trace::config()
                        .with_default_sampler(Sampler::AlwaysOn)
                        .with_id_generator(IdGenerator::default())
                        .with_max_events_per_span(64)
                        .with_max_attributes_per_span(16)
                        .with_max_events_per_span(16)
                        .with_resource(Resource::new(vec![])),
                )
                .install()?;

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
