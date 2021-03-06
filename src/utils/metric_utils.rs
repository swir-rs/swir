use super::config::*;
use std::sync::Arc;
use tokio::time::sleep;

use futures::Stream;
use futures::StreamExt;
use http::StatusCode;
use http::{Request, Response};
use hyper::{Body, Request as HyperRequest, Response as HyperResponse};
use opentelemetry::global;
use opentelemetry::metrics::{Descriptor, InstrumentKind};
use opentelemetry::sdk::{
    export::metrics::{Aggregator, AggregatorSelector},
    metrics::{aggregators, selectors, PushController},
};
use opentelemetry::KeyValue;
use opentelemetry_otlp::ExporterConfig;
use opentelemetry_otlp::Protocol;

use std::convert::TryInto;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime};
use tonic::transport::Channel;
use tonic::{body::BoxBody, transport::NamedService};
use tower::Service;

fn delayed_interval(duration: Duration) -> impl Stream<Item = tokio::time::Instant> {
    opentelemetry::util::tokio_interval_stream(duration).skip(1)
}

#[derive(Debug, Clone)]
pub struct Counters {
    pub request_counter: opentelemetry::metrics::Counter<u64>,
    pub success_counter: opentelemetry::metrics::Counter<u64>,
    pub client_error_counter: opentelemetry::metrics::Counter<u64>,
    pub server_error_counter: opentelemetry::metrics::Counter<u64>,
}

#[derive(Debug, Clone)]
pub struct ResourceObservers {
    pub vsize: opentelemetry::metrics::ValueRecorder<u64>,
    pub rss: opentelemetry::metrics::ValueRecorder<u64>,
    pub threads: opentelemetry::metrics::ValueRecorder<u64>,
    pub utime: opentelemetry::metrics::ValueRecorder<u64>,
    pub stime: opentelemetry::metrics::ValueRecorder<u64>,
    pub cutime: opentelemetry::metrics::ValueRecorder<u64>,
    pub cstime: opentelemetry::metrics::ValueRecorder<u64>,
}

#[derive(Debug, Clone)]
pub struct Histograms {
    pub request_response_time: opentelemetry::metrics::ValueRecorder<f64>,
}

#[derive(Debug, Clone)]
pub struct InOutMetricInstruments {
    pub labels: Vec<KeyValue>,
    pub incoming_counters: Counters,
    pub outgoing_counters: Counters,
    pub incoming_histograms: Histograms,
    pub outgoing_histograms: Histograms,
}

#[derive(Debug, Clone)]
pub struct OutMetricInstruments {
    pub labels: Vec<KeyValue>,
    pub outgoing_counters: Counters,
    pub outgoing_histograms: Histograms,
}

#[derive(Debug, Clone)]
pub struct SystemInstruments {
    pub labels: Vec<KeyValue>,
    pub observers: ResourceObservers,
}

#[derive(Debug, Clone)]
pub struct MetricRegistry {
    pub http: InOutMetricInstruments,
    pub grpc: InOutMetricInstruments,
    pub kafka: InOutMetricInstruments,
    pub nats: InOutMetricInstruments,
    pub redis: OutMetricInstruments,
    pub system: SystemInstruments,
}

pub fn bump_http_response_counters(status: &StatusCode, counters: &Counters, labels: &[KeyValue]) {
    if status.is_server_error() {
        counters.server_error_counter.add(1, &labels);
    }

    if status.is_client_error() {
        counters.client_error_counter.add(1, &labels);
    }

    if status.is_success() {
        counters.success_counter.add(1, &labels);
    }
}

pub fn init_metrics(config: &Swir) -> Result<(MetricRegistry, Option<PushController>), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut controller = None;
    let mut name = "swir.com".to_string();
    if let Some(cfg) = &config.tracing {
        if let Some(open_telemetry) = &cfg.open_telemetry {
            let export_config = ExporterConfig {
                endpoint: format!("grpc://{}:{}", open_telemetry.collector_address, open_telemetry.collector_port),
                protocol: Protocol::Grpc,
                ..ExporterConfig::default()
            };
            name = open_telemetry.service_name.clone();
            controller = Some(
                // }
                opentelemetry_otlp::new_metrics_pipeline(tokio::spawn, delayed_interval)
                    .with_export_config(export_config)
                    .with_period(std::time::Duration::from_secs(open_telemetry.metric_window.unwrap_or(30)))
                    .with_aggregator_selector(selectors::simple::Selector::Histogram(vec![0.0, 0.1, 0.2, 0.3, 0.5, 0.8, 1.3, 2.1]))
                    .build()?,
            );
        }
    }

    let meter = global::meter("swir");
    let labels = vec![KeyValue::new("name", name)];

    let http_incoming_counters = Counters {
        request_counter: meter.u64_counter("http_incoming_requests").with_description("Total number of HTTP requests made.").init(),
        success_counter: meter.u64_counter("http_incoming_success").with_description("Total number of success").init(),
        client_error_counter: meter.u64_counter("http_incoming_clienterrors").with_description("Total number of client errors").init(),
        server_error_counter: meter.u64_counter("http_incoming_servererrors").with_description("Total number of server errors").init(),
    };
    let http_incoming_histograms = Histograms {
        request_response_time: meter
            .f64_value_recorder("http_incoming_request_duration_seconds")
            .with_description("The HTTP request latencies in seconds.")
            .init(),
    };

    let http_outgoing_counters = Counters {
        request_counter: meter.u64_counter("http_outgoing_requests").with_description("Total number of HTTP requests made.").init(),
        success_counter: meter.u64_counter("http_outgoing_success").with_description("Total number of success").init(),
        client_error_counter: meter.u64_counter("http_outgoing_clienterrors").with_description("Total number of client errors").init(),
        server_error_counter: meter.u64_counter("http_outgoing_servererrors").with_description("Total number of server errors").init(),
    };
    let http_outgoing_histograms = Histograms {
        request_response_time: meter
            .f64_value_recorder("http_outgoing_request_duration_seconds")
            .with_description("The HTTP request latencies in seconds.")
            .init(),
    };

    let grpc_incoming_counters = Counters {
        request_counter: meter.u64_counter("grpc_incoming_requests").with_description("Total number of GRPC requests made.").init(),
        success_counter: meter.u64_counter("grpc_incoming_success").with_description("Total number of success").init(),
        client_error_counter: meter.u64_counter("grpc_incoming_clienterrors").with_description("Total number of client errors").init(),
        server_error_counter: meter.u64_counter("grpc_incoming_servererrors").with_description("Total number of server errors").init(),
    };
    let grpc_incoming_histograms = Histograms {
        request_response_time: meter
            .f64_value_recorder("grpc_incoming_request_duration_seconds")
            .with_description("The GRPC request latencies in seconds.")
            .init(),
    };

    let grpc_outgoing_counters = Counters {
        request_counter: meter.u64_counter("grpc_outgoing_requests").with_description("Total number of GRPC requests made.").init(),
        success_counter: meter.u64_counter("grpc_outgoing_success").with_description("Total number of success").init(),
        client_error_counter: meter.u64_counter("grpc_outgoing_clienterrors").with_description("Total number of client errors").init(),
        server_error_counter: meter.u64_counter("grpc_outgoing_servererrors").with_description("Total number of server errors").init(),
    };
    let grpc_outgoing_histograms = Histograms {
        request_response_time: meter
            .f64_value_recorder("grpc_outgoing_request_duration_seconds")
            .with_description("The GRPC request latencies in seconds.")
            .init(),
    };

    let kafka_incoming_counters = Counters {
        request_counter: meter.u64_counter("kafka_incoming_requests").with_description("Total number of KAFKA requests made.").init(),
        success_counter: meter.u64_counter("kafka_incoming_success").with_description("Total number of success").init(),
        client_error_counter: meter.u64_counter("kafka_incoming_clienterrors").with_description("Total number of client errors").init(),
        server_error_counter: meter.u64_counter("kafka_incoming_servererrors").with_description("Total number of server errors").init(),
    };
    let kafka_incoming_histograms = Histograms {
        request_response_time: meter
            .f64_value_recorder("kafka_incoming_request_duration_seconds")
            .with_description("The KAFKA request latencies in seconds.")
            .init(),
    };

    let kafka_outgoing_counters = Counters {
        request_counter: meter.u64_counter("kafka_outgoing_requests").with_description("Total number of KAFKA requests made.").init(),
        success_counter: meter.u64_counter("kafka_outgoing_success").with_description("Total number of success").init(),
        client_error_counter: meter.u64_counter("kafka_outgoing_clienterrors").with_description("Total number of client errors").init(),
        server_error_counter: meter.u64_counter("kafka_outgoing_servererrors").with_description("Total number of server errors").init(),
    };

    let kafka_outgoing_histograms = Histograms {
        request_response_time: meter
            .f64_value_recorder("kafka_outgoing_request_duration_seconds")
            .with_description("The KAFKA request latencies in seconds.")
            .init(),
    };

    let nats_incoming_counters = Counters {
        request_counter: meter.u64_counter("nats_incoming_requests").with_description("Total number of NATS requests made.").init(),
        success_counter: meter.u64_counter("nats_incoming_success").with_description("Total number of success").init(),
        client_error_counter: meter.u64_counter("nats_incoming_clienterrors").with_description("Total number of client errors").init(),
        server_error_counter: meter.u64_counter("nats_incoming_servererrors").with_description("Total number of server errors").init(),
    };
    let nats_incoming_histograms = Histograms {
        request_response_time: meter
            .f64_value_recorder("nats_incoming_request_duration_seconds")
            .with_description("The NATS request latencies in seconds.")
            .init(),
    };

    let nats_outgoing_counters = Counters {
        request_counter: meter.u64_counter("nats_outgoing_requests").with_description("Total number of NATS requests made.").init(),
        success_counter: meter.u64_counter("nats_outgoing_success").with_description("Total number of success").init(),
        client_error_counter: meter.u64_counter("nats_outgoing_clienterrors").with_description("Total number of client errors").init(),
        server_error_counter: meter.u64_counter("nats_outgoing_servererrors").with_description("Total number of server errors").init(),
    };
    let nats_outgoing_histograms = Histograms {
        request_response_time: meter
            .f64_value_recorder("nats_outgoing_request_duration_seconds")
            .with_description("The NATS request latencies in seconds.")
            .init(),
    };

    let redis_outgoing_counters = Counters {
        request_counter: meter.u64_counter("redis_outgoing_requests").with_description("Total number of REDIS requests made.").init(),
        success_counter: meter.u64_counter("redis_outgoing_success").with_description("Total number of success").init(),
        client_error_counter: meter.u64_counter("redis_outgoing_clienterrors").with_description("Total number of client errors").init(),
        server_error_counter: meter.u64_counter("redis_outgoing_servererrors").with_description("Total number of server errors").init(),
    };
    let redis_outgoing_histograms = Histograms {
        request_response_time: meter
            .f64_value_recorder("redis_outgoing_request_duration_seconds")
            .with_description("The REDIS request latencies in seconds.")
            .init(),
    };

    let resource_observers = ResourceObservers {
        vsize: meter.u64_value_recorder("process_vsize").with_description("Virtual memory size in bytes").init(),
        rss: meter
            .u64_value_recorder("process_rss")
            .with_description("Resident Set Size: number of pages the process has in real memory in pages")
            .init(),
        threads: meter.u64_value_recorder("process_threads").with_description("Number of threads in this process").init(),
        utime: meter
            .u64_value_recorder("process_utime")
            .with_description("Amount of time that this process has been scheduled in user mode, measured in clock ticks")
            .init(),
        stime: meter
            .u64_value_recorder("process_stime")
            .with_description("Amount of time that this process has been scheduled in kernel mode, measured in clock ticks")
            .init(),
        cutime: meter
            .u64_value_recorder("process_cutime")
            .with_description("Amount of time that this process's waited-for children have been scheduled in user mode, measured in clock ticks")
            .init(),
        cstime: meter
            .u64_value_recorder("process_cstime")
            .with_description("Amount of time that this process's waited-for children have been scheduled in kernel mode, measured in clock ticks")
            .init(),
    };

    let mr = MetricRegistry {
        http: InOutMetricInstruments {
            labels: labels.clone(),
            incoming_counters: http_incoming_counters,
            outgoing_counters: http_outgoing_counters,
            incoming_histograms: http_incoming_histograms,
            outgoing_histograms: http_outgoing_histograms,
        },
        grpc: InOutMetricInstruments {
            labels: labels.clone(),
            incoming_counters: grpc_incoming_counters,
            outgoing_counters: grpc_outgoing_counters,
            incoming_histograms: grpc_incoming_histograms,
            outgoing_histograms: grpc_outgoing_histograms,
        },
        kafka: InOutMetricInstruments {
            labels: labels.clone(),
            incoming_counters: kafka_incoming_counters,
            outgoing_counters: kafka_outgoing_counters,
            incoming_histograms: kafka_incoming_histograms,
            outgoing_histograms: kafka_outgoing_histograms,
        },
        nats: InOutMetricInstruments {
            labels: labels.clone(),
            incoming_counters: nats_incoming_counters,
            outgoing_counters: nats_outgoing_counters,
            incoming_histograms: nats_incoming_histograms,
            outgoing_histograms: nats_outgoing_histograms,
        },
        redis: OutMetricInstruments {
            labels: labels.clone(),
            outgoing_counters: redis_outgoing_counters,
            outgoing_histograms: redis_outgoing_histograms,
        },
        system: SystemInstruments {
            labels,
            observers: resource_observers,
        },
    };
    Ok((mr, controller))
}

#[derive(Debug, Clone)]
pub struct MeteredService<S> {
    pub inner: S,
    pub labels: Vec<KeyValue>,
    pub counters: Counters,
    pub histograms: Histograms,
}

impl<S> Service<HyperRequest<Body>> for MeteredService<S>
where
    S: Service<HyperRequest<Body>, Response = HyperResponse<BoxBody>> + NamedService + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: HyperRequest<Body>) -> Self::Future {
        let method = req.method().clone();
        let uri = req.uri().clone();
        let mut svc = self.inner.clone();
        let counters = self.counters.clone();
        let mut labels = self.labels.clone();
        labels.push(KeyValue::new("method", method.to_string()));
        labels.push(KeyValue::new("uri", uri.to_string()));
        let histograms = self.histograms.clone();
        Box::pin(async move {
            counters.request_counter.add(1, &labels);
            let request_start = SystemTime::now();
            let response = svc.call(req).await;
            if let Ok(resp) = &response {
                bump_http_response_counters(&resp.status(), &counters, &labels);
            }

            histograms.request_response_time.record(request_start.elapsed().map_or(0.0, |d| d.as_secs_f64()), &labels);
            response
        })
    }
}

impl<S: NamedService> NamedService for MeteredService<S> {
    const NAME: &'static str = S::NAME;
}

#[derive(Debug, Clone)]
pub struct MeteredClientService {
    pub inner: Channel,
    pub labels: Vec<KeyValue>,
    pub counters: Counters,
    pub histograms: Histograms,
}

#[derive(Debug, Clone)]
struct Selector {}

impl AggregatorSelector for Selector {
    fn aggregator_for(&self, descriptor: &Descriptor) -> Option<Arc<dyn Aggregator + Send + Sync>> {
        let boundaries = vec![0.0, 0.1, 0.2, 0.3, 0.5, 0.8, 1.3, 2.1];
        match descriptor.instrument_kind() {
            InstrumentKind::ValueObserver => Some(Arc::new(aggregators::last_value())),
            InstrumentKind::ValueRecorder => Some(Arc::new(aggregators::histogram(descriptor, &boundaries))),
            _ => Some(Arc::new(aggregators::sum())),
        }
    }
}

pub async fn self_metrics(metric_registry: Arc<MetricRegistry>) {
    let observers = metric_registry.system.observers.clone();
    let labels = metric_registry.system.labels.clone();
    loop {
        sleep(Duration::from_secs(10)).await;
        let maybe_stats = procinfo::pid::stat_self();
        if let Ok(stats) = maybe_stats {
            observers.vsize.record(stats.vsize.try_into().unwrap(), &labels);
            observers.rss.record(stats.rss.try_into().unwrap(), &labels);
            observers.threads.record(stats.num_threads.try_into().unwrap(), &labels);
            observers.utime.record(stats.utime.try_into().unwrap(), &labels);
            observers.stime.record(stats.stime.try_into().unwrap(), &labels);
            observers.cutime.record(stats.cutime.try_into().unwrap(), &labels);
            observers.cstime.record(stats.cstime.try_into().unwrap(), &labels);
        } else {
            warn!("Can't get metrics {:?}", maybe_stats);
        }
    }
}

type PinBoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

impl Service<Request<BoxBody>> for MeteredClientService {
    type Response = Response<Body>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = PinBoxFuture<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: Request<BoxBody>) -> Self::Future {
        let clone = self.inner.clone();

        // take the service that was ready
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let counters = self.counters.clone();
        let histograms = self.histograms.clone();
        let mut labels = self.labels.clone();

        Box::pin(async move {
            let method = req.method().clone();
            let uri = req.uri().clone();
            labels.push(KeyValue::new("method", method.to_string()));
            labels.push(KeyValue::new("uri", uri.to_string()));

            counters.request_counter.add(1, &labels);
            let request_start = SystemTime::now();
            let result = inner.call(req).await.map_err(Into::into);
            if let Ok(hyper_response) = &result {
                bump_http_response_counters(&hyper_response.status(), &counters, &labels);
            }
            histograms.request_response_time.record(request_start.elapsed().map_or(0.0, |d| d.as_secs_f64()), &labels);
            result
        })
    }
}
