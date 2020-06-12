
use opentelemetry::api;
use tracing::{Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use opentelemetry::api::context::propagation::text_propagator::HttpTextFormat;
use tonic::metadata::AsciiMetadataValue;
use http::{header::HeaderName, HeaderMap};

pub fn get_tracing_header()->Option<(&'static str, String)>{
    let span = Span::current();	
    let span_ctx = span.context();
    let propagator = api::TraceContextPropagator::new();
    let mut carrier = std::collections::HashMap::new();
    propagator.inject_context(&span_ctx,&mut carrier);
    carrier.get("traceparent").map(|f| ("traceparent",f.to_string()))
}

pub fn get_grpc_tracing_header()->Option<(&'static str, AsciiMetadataValue)>{
    get_tracing_header().map(|f| (f.0,AsciiMetadataValue::from_str(&f.1).unwrap()))
}


pub fn from_http_headers(header_map:&HeaderMap)->Option<api::Context>{
    let trace_header = HeaderName::from_lowercase("traceparent".as_bytes()).unwrap();				
    let maybe_tracingheader_header = header_map.get(trace_header);
    maybe_tracingheader_header.map(|value| {
	let propagator = api::TraceContextPropagator::new();
	let mut carrier = std::collections::HashMap::new();	carrier.insert("traceparent".to_string(),String::from_utf8_lossy(value.as_bytes()).to_string());
	propagator.extract(&carrier)
    }
    )
}
