//#![deny(warnings)]
extern crate custom_error;
#[macro_use]
extern crate tracing;

mod frontend_handlers;
mod messaging_handlers;
mod persistence_handlers;
mod si_handlers;
mod utils;

use futures::StreamExt;
use std::iter::Iterator;

use si_handlers::si_http_handler;
mod service_discovery;

pub mod swir_grpc_internal_api {
    tonic::include_proto!("swir_internal");
}
pub mod swir_common {
    tonic::include_proto!("swir_common");
}

pub mod swir_grpc_api {
    tonic::include_proto!("swir_public");
}

use std::{fs, net::SocketAddr, sync::Arc};

use crate::metric_utils::bump_http_response_counters;
use crate::utils::{config::*, metric_utils, tracing_utils};
use frontend_handlers::http_handler::{client_handler, handler};
use frontend_handlers::{grpc_handler, grpc_internal_handler};
use http::header::HeaderName;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Server};
use opentelemetry::KeyValue;
use std::time::SystemTime;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tonic::transport::{Certificate, Identity, Server as TonicServer, ServerTlsConfig};
use tracing::field;
use tracing_futures::Instrument;
use utils::metric_utils::MetricRegistry;
use utils::pki_utils::{load_certs, load_private_key};

static X_CORRRELATION_ID_HEADER_NAME: &str = "x-correlation-id";

#[tokio::main(worker_threads = 8)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let swir_config = Swir::new();
    println!("{:?}", swir_config);
    let _tracer = tracing_utils::init_tracer(&swir_config)?;
    let (metric_registry, _controller) = metric_utils::init_metrics(&swir_config)?;
    let metric_registry = Arc::new(metric_registry);

    let mc: MemoryChannels = utils::config::create_memory_channels(&swir_config);
    let ip = swir_config.ip.clone();

    let http_port: u16 = swir_config.http_port;
    let grpc_port: u16 = swir_config.grpc_port;
    let internal_grpc_port: u16 = swir_config.internal_grpc_port;
    let client_executable = swir_config.client_executable.clone();

    let http_addr = SocketAddr::new(ip.parse().unwrap(), http_port);
    let grpc_addr = SocketAddr::new(ip.parse().unwrap(), grpc_port);
    let internal_grpc_addr = SocketAddr::new(ip.parse().unwrap(), internal_grpc_port);

    let mut tasks = vec![];
    tasks.append(&mut start_rest_client_service(&swir_config, &mc, metric_registry.clone()));
    tasks.append(&mut start_client_http_interface(&http_addr, &swir_config, &mc, metric_registry.clone()));
    tasks.append(&mut start_client_grpc_interface(&grpc_addr, &swir_config, &mc, metric_registry.clone()));
    tasks.append(&mut start_internal_grpc_interface(&internal_grpc_addr, &swir_config, &mc, metric_registry.clone()));
    tasks.append(&mut start_service_invocation_service(&swir_config, &mc, metric_registry.clone()));

    tasks.append(&mut start_service_invocation_service_private_http_interface(&swir_config, &mc, metric_registry.clone()));

    tasks.append(&mut start_pubsub_service(&swir_config, mc.messaging_memory_channels, metric_registry.clone()));
    tasks.append(&mut start_persistence_service(&swir_config, mc.persistence_memory_channels, metric_registry.clone()));
    tasks.append(&mut vec![tokio::spawn(metric_utils::self_metrics(metric_registry.clone()))]);

    if let Some(command) = client_executable {
        utils::command_utils::run_java_command(command);
    }
    futures::future::join_all(tasks).await;
    Ok(())
}

fn start_client_http_interface(http_addr: &SocketAddr, swir_config: &Swir, mc: &MemoryChannels, metric_registry: Arc<MetricRegistry>) -> Vec<tokio::task::JoinHandle<()>> {
    let mut tasks = vec![];
    let http_addr = *http_addr;
    let mmc = &mc.messaging_memory_channels;
    let pmc = &mc.persistence_memory_channels;
    let simc = &mc.si_memory_channels;
    let to_client_sender_for_rest = mmc.to_client_sender_for_rest.clone();
    let from_client_to_messaging_sender = mmc.from_client_to_messaging_sender.clone();
    let from_client_to_persistence_senders = pmc.from_client_to_persistence_senders.clone();
    let client_sender_for_http = simc.client_sender.clone();
    let client_sender_for_https = simc.client_sender.clone();
    let http_metric_registry = metric_registry.to_owned();

    let http_service = make_service_fn(move |_| {
        let from_client_to_messaging_sender = from_client_to_messaging_sender.clone();
        let from_client_to_persistence_senders = from_client_to_persistence_senders.clone();
        let to_client_sender_for_rest = to_client_sender_for_rest.clone();
        let client_sender_for_http = client_sender_for_http.to_owned();
        let metric_registry = http_metric_registry.to_owned();

        async move {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                let from_client_to_messaging_sender = from_client_to_messaging_sender.clone();
                let from_client_to_persistence_senders = from_client_to_persistence_senders.clone();
                let to_client_sender_for_rest = to_client_sender_for_rest.clone();
                let client_sender_for_http = client_sender_for_http.to_owned();
                let metric_registry = metric_registry.to_owned();

                Box::pin(async move {
                    let method = req.method();
                    let uri = req.uri();
                    let mut labels = metric_registry.http.labels.clone();
                    labels.push(KeyValue::new("method", method.to_string()));
                    labels.push(KeyValue::new("uri", uri.to_string()));
                    let counters = &metric_registry.http.incoming_counters;
                    let histograms = &metric_registry.http.incoming_histograms;

                    counters.request_counter.add(1, &labels);
                    let request_start = SystemTime::now();
                    let response = handler(
                        req,
                        from_client_to_messaging_sender,
                        to_client_sender_for_rest,
                        from_client_to_persistence_senders,
                        client_sender_for_http,
                    )
                    .await;
                    if let Ok(resp) = &response {
                        bump_http_response_counters(&resp.status(), &counters, &labels);
                    };
                    histograms.request_response_time.record(request_start.elapsed().map_or(0.0, |d| d.as_secs_f64()), &labels);
                    response
                })
            }))
        }
    });

    let to_client_sender_for_rest = mmc.to_client_sender_for_rest.clone();
    let from_client_to_messaging_sender = mmc.from_client_to_messaging_sender.clone();
    let from_client_to_persistence_senders = pmc.from_client_to_persistence_senders.clone();
    let https_metric_registry = metric_registry;
    let https_service = make_service_fn(move |_| {
        let from_client_to_messaging_sender = from_client_to_messaging_sender.clone();
        let to_client_sender_for_rest = to_client_sender_for_rest.clone();
        let from_client_to_persistence_senders = from_client_to_persistence_senders.clone();
        let client_sender_for_https = client_sender_for_https.to_owned();
        let metric_registry = https_metric_registry.to_owned();

        async move {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                let from_client_to_messaging_sender = from_client_to_messaging_sender.clone();
                let from_client_to_persistence_senders = from_client_to_persistence_senders.clone();
                let to_client_sender_for_rest = to_client_sender_for_rest.clone();
                let client_sender_for_https = client_sender_for_https.to_owned();
                let metric_registry = metric_registry.to_owned();

                Box::pin(async move {
                    let method = req.method();
                    let uri = req.uri();
                    let mut labels = metric_registry.http.labels.clone();
                    labels.push(KeyValue::new("method", method.to_string()));
                    labels.push(KeyValue::new("uri", uri.to_string()));
                    let counters = &metric_registry.http.incoming_counters;
                    let histograms = &metric_registry.http.incoming_histograms;

                    counters.request_counter.add(1, &labels);
                    let request_start = SystemTime::now();
                    let response = handler(
                        req,
                        from_client_to_messaging_sender,
                        to_client_sender_for_rest,
                        from_client_to_persistence_senders,
                        client_sender_for_https,
                    )
                    .await;
                    if let Ok(resp) = &response {
                        bump_http_response_counters(&resp.status(), &counters, &labels);
                    };
                    histograms.request_response_time.record(request_start.elapsed().map_or(0.0, |d| d.as_secs_f64()), &labels);
                    response
                })
            }))
        }
    });

    let span = tracing::info_span!("SWIR_CLIENT_HTTP_API", correlation_id = field::Empty, origin = field::Empty);
    let _sp = span.enter();

    if let Some(tls_config) = swir_config.tls_config.clone() {
        let http_tls_certificate = tls_config.certificate;
        let http_tls_key = tls_config.private_key;
        let certs = load_certs(http_tls_certificate).unwrap();
        let key = load_private_key(http_tls_key).unwrap();
        let span = span.clone();
        let https_client_interface = tokio::spawn(async move {
            let mut config = rustls::ServerConfig::new(rustls::NoClientAuth::new());
            config.set_single_cert(certs, key).expect("invalid key or certificate");
            let tls_acceptor = TlsAcceptor::from(Arc::new(config));
            let arc_acceptor = Arc::new(tls_acceptor);
            let listener = TcpListener::bind(&http_addr).await.unwrap();

            let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

            let incoming = hyper::server::accept::from_stream(incoming.filter_map(|socket| async {
                match socket {
                    Ok(stream) => match arc_acceptor.clone().accept(stream).await {
                        Ok(val) => Some(Ok::<_, hyper::Error>(val)),
                        Err(e) => {
                            println!("TLS error: {}", e);
                            None
                        }
                    },
                    Err(e) => {
                        println!("TCP socket error: {}", e);
                        None
                    }
                }
            }));

            let res = Server::builder(incoming).serve(https_service).instrument(span).await;
            if let Err(e) = res {
                warn!("Problem starting HTTPs interface {:?}", e);
            }
        });

        tasks.push(https_client_interface);
    } else {
        let span = span.clone();
        let http_client_interface = tokio::spawn(async move {
            let res = Server::bind(&http_addr).serve(http_service).instrument(span).await;
            if let Err(e) = res {
                warn!("Problem starting HTTP interface {:?}", e);
            }
        });
        tasks.push(http_client_interface);
    }
    tasks
}

fn start_client_grpc_interface(grpc_addr: &SocketAddr, swir_config: &Swir, mc: &MemoryChannels, metric_registry: Arc<MetricRegistry>) -> Vec<tokio::task::JoinHandle<()>> {
    let mut tasks = vec![];
    let grpc_addr = *grpc_addr;
    let mmc = &mc.messaging_memory_channels;

    let pmc = &mc.persistence_memory_channels;
    let simc = &mc.si_memory_channels;

    let (to_client_sender, to_client_receiver) = (mmc.to_client_sender_for_grpc.clone(), mmc.to_client_receiver_for_grpc.clone());

    let from_client_to_messaging_sender = mmc.from_client_to_messaging_sender.clone();

    let from_client_to_persistence_senders = pmc.from_client_to_persistence_senders.clone();
    let client_sender_for_public = simc.client_sender.clone();
    let tls_config = swir_config.tls_config.clone();
    let incoming_metric_registry = metric_registry.to_owned();
    let grpc_client_interface = tokio::spawn(async move {
        let metric_registry = incoming_metric_registry.to_owned();
        let pub_sub_handler = grpc_handler::SwirPubSubApi::new(from_client_to_messaging_sender.clone(), to_client_sender, metric_registry.clone());
        let persistence_handler = grpc_handler::SwirPersistenceApi::new(from_client_to_persistence_senders, metric_registry.clone());

        let pub_sub_svc = swir_grpc_api::pub_sub_api_server::PubSubApiServer::new(pub_sub_handler);
        let persistence_svc = swir_grpc_api::persistence_api_server::PersistenceApiServer::new(persistence_handler);
        let service_invocation_handler = grpc_handler::SwirServiceInvocationApi::new(client_sender_for_public, metric_registry.clone());
        let service_invocation_svc = swir_grpc_api::service_invocation_api_server::ServiceInvocationApiServer::new(service_invocation_handler);

        let builder = if let Some(tls_config) = tls_config {
            let cert = fs::read(tls_config.certificate).unwrap();
            let key = fs::read(tls_config.private_key).unwrap();
            let identity = Identity::from_pem(cert, key);
            TonicServer::builder().tls_config(ServerTlsConfig::new().identity(identity))
        } else {
            Ok(TonicServer::builder())
        };

        if let Ok(builder) = builder {
            let grpc = builder
                .trace_fn(|header_map| {
                    let span = tracing::info_span!("CLIENT_GRPC_INCOMING", correlation_id = field::Empty, origin = field::Empty);
                    let span = tracing_utils::from_http_headers(span, &header_map);
                    let corr_id_header = HeaderName::from_lowercase(X_CORRRELATION_ID_HEADER_NAME.as_bytes()).unwrap();
                    let origin_header = HeaderName::from_lowercase("origin".as_bytes()).unwrap();
                    let maybe_corr_id_header = header_map.get(corr_id_header);
                    let maybe_origin_header = header_map.get(origin_header);

                    if let Some(value) = maybe_corr_id_header {
                        span.record("correlation_id", &String::from_utf8_lossy(value.as_bytes()).to_string().as_str());
                    };
                    if let Some(value) = maybe_origin_header {
                        span.record("origin", &String::from_utf8_lossy(value.as_bytes()).to_string().as_str());
                    };
                    span
                })
                .add_service(metric_utils::MeteredService {
                    inner: pub_sub_svc,
                    labels: metric_registry.grpc.labels.clone(),
                    counters: metric_registry.grpc.incoming_counters.clone(),
                    histograms: metric_registry.grpc.incoming_histograms.clone(),
                })
                .add_service(metric_utils::MeteredService {
                    inner: persistence_svc,
                    labels: metric_registry.grpc.labels.clone(),
                    counters: metric_registry.grpc.incoming_counters.clone(),
                    histograms: metric_registry.grpc.incoming_histograms.clone(),
                })
                .add_service(metric_utils::MeteredService {
                    inner: service_invocation_svc,
                    labels: metric_registry.grpc.labels.clone(),
                    counters: metric_registry.grpc.incoming_counters.clone(),
                    histograms: metric_registry.grpc.incoming_histograms.clone(),
                })
                .serve(grpc_addr.to_owned());

            let res = grpc.await;
            if let Err(e) = res {
                warn!("Problem starting gRPC interface {:?}", e);
            }
        }
    });
    let client_config = swir_config.client_config.clone();
    let grpc_notification_interface = tokio::spawn(async move {
        let client_config = client_config.clone();
        if let Some(client_config) = client_config {
            info!("Starting GRPC notification interface {:?}", client_config);
            grpc_handler::client_handler(client_config, to_client_receiver, metric_registry.clone()).await;
        }
    });
    tasks.push(grpc_client_interface);
    tasks.push(grpc_notification_interface);
    tasks
}

fn start_internal_grpc_interface(grpc_addr: &SocketAddr, swir_config: &Swir, mc: &MemoryChannels, metric_registry: Arc<MetricRegistry>) -> Vec<tokio::task::JoinHandle<()>> {
    let mut tasks = vec![];
    let grpc_addr = *grpc_addr;

    let simc = &mc.si_memory_channels;
    let si_config = swir_config.services.clone();
    let client_sender_for_internal = simc.client_sender.clone();

    let grpc_internal_interface = tokio::spawn(async move {
        if let Some(services) = si_config {
            let tls_config = services.tls_config;
            let service_invocation_handler = grpc_internal_handler::SwirServiceInvocationDiscoveryApi::new(client_sender_for_internal);
            let service_invocation_svc = swir_grpc_internal_api::service_invocation_discovery_api_server::ServiceInvocationDiscoveryApiServer::new(service_invocation_handler);

            let svc = metric_utils::MeteredService {
                inner: service_invocation_svc,
                labels: metric_registry.grpc.labels.clone(),
                counters: metric_registry.grpc.incoming_counters.clone(),
                histograms: metric_registry.grpc.incoming_histograms.clone(),
            };

            let cert = fs::read(tls_config.server_cert).unwrap();
            let key = fs::read(tls_config.server_key).unwrap();
            let server_identity = Identity::from_pem(cert, key);
            let client_ca_cert = fs::read(tls_config.client_ca_cert).unwrap();
            let client_ca_cert = Certificate::from_pem(client_ca_cert);

            let tls = ServerTlsConfig::new().identity(server_identity).client_ca_root(client_ca_cert);
            let grpc = TonicServer::builder()
                .tls_config(tls)
                .unwrap()
                .trace_fn(|header_map| {
                    let corr_id_header = HeaderName::from_lowercase(X_CORRRELATION_ID_HEADER_NAME.as_bytes()).unwrap();
                    let origin_header = HeaderName::from_lowercase("origin".as_bytes()).unwrap();
                    let span = tracing::info_span!("INTERNAL_GRPC", correlation_id = field::Empty, origin = field::Empty);
                    let span = tracing_utils::from_http_headers(span, &header_map);

                    let maybe_corr_id_header = header_map.get(corr_id_header);
                    let maybe_origin_header = header_map.get(origin_header);

                    if let Some(value) = maybe_corr_id_header {
                        span.record("correlation_id", &String::from_utf8_lossy(value.as_bytes()).to_string().as_str());
                    };
                    if let Some(value) = maybe_origin_header {
                        span.record("origin", &String::from_utf8_lossy(value.as_bytes()).to_string().as_str());
                    };

                    span
                })
                .add_service(svc)
                .serve(grpc_addr);

            let res = grpc.await;
            if let Err(e) = res {
                warn!("Problem starting gRPC interface {:?}", e);
            }
        }
    });

    tasks.push(grpc_internal_interface);
    tasks
}

fn start_service_invocation_service(swir_config: &Swir, mc: &MemoryChannels, metric_registry: Arc<MetricRegistry>) -> Vec<tokio::task::JoinHandle<()>> {
    let mut tasks = vec![];
    let config = swir_config.services.clone();
    let internal_grpc_port = swir_config.internal_grpc_port;

    let mmc = &mc.messaging_memory_channels;

    let simc = &mc.si_memory_channels;
    let to_si_http_client = mmc.to_client_sender_for_rest.clone();
    let receiver = simc.receiver.clone();
    let si = tokio::spawn(async move {
        if let Some(services) = config {
            match services.resolver.resolver_type {
                ResolverType::MDNS => {
                    if let Ok(resolver) = service_discovery::mdns_sd::MDNSServiceDiscovery::new(internal_grpc_port) {
                        si_handlers::ServiceInvocationService::new(metric_registry)
                            .start(services, &resolver, receiver, to_si_http_client)
                            .await;
                    } else {
                        warn!("Problem with resolver");
                    };
                }
                ResolverType::DynamoDb => {
                    if let Some(resolver_config) = &services.resolver.resolver_config {
                        let region = resolver_config.get("region");
                        let table = resolver_config.get("table");
                        if let (Some(r), Some(t)) = (region, table) {
                            if let Ok(resolver) = service_discovery::dynamodb_sd::DynamoDBServiceDiscovery::new(r.to_string(), t.to_string(), internal_grpc_port) {
                                si_handlers::ServiceInvocationService::new(metric_registry)
                                    .start(services, &resolver, receiver, to_si_http_client)
                                    .await;
                            } else {
                                warn!("Problem with resolver");
                            };
                        } else {
                            warn!("Problem with resolver: Invalid resolver config");
                        }
                    }
                }
            }
        }
    });
    tasks.push(si);
    tasks
}

fn start_service_invocation_service_private_http_interface(swir_config: &Swir, mc: &MemoryChannels, metric_registry: Arc<MetricRegistry>) -> Vec<tokio::task::JoinHandle<()>> {
    let mut tasks = vec![];
    let config = swir_config.services.clone();
    let simc = &mc.si_memory_channels;
    let client_sender = simc.client_sender.clone();
    let si_private_interface = tokio::spawn(async move {
        if let Some(services) = config {
            if let Some(private_http_socket) = services.private_http_socket {
                info!("Private invocation service enabled at {}", private_http_socket);
                let http_service = make_service_fn(move |_| {
                    let client_sender_for_http = client_sender.to_owned();
                    let metric_registry = metric_registry.to_owned();
                    async move {
                        let client_sender_for_http = client_sender_for_http.to_owned();
                        Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                            let client_sender_for_http = client_sender_for_http.to_owned();
                            let metric_registry = metric_registry.to_owned();
                            let mut labels = metric_registry.http.labels.clone();
                            let counters = metric_registry.http.incoming_counters.clone();
                            let histograms = metric_registry.http.incoming_histograms.clone();

                            Box::pin(async move {
                                let method = req.method().clone();
                                let uri = req.uri().clone();
                                labels.push(KeyValue::new("method", method.to_string()));
                                labels.push(KeyValue::new("uri", uri.to_string()));
                                counters.request_counter.add(1, &labels);
                                let request_start = SystemTime::now();
                                let response = si_http_handler::handler(req, client_sender_for_http.to_owned()).await;
                                if let Ok(resp) = &response {
                                    bump_http_response_counters(&resp.status(), &counters, &labels);
                                };
                                histograms.request_response_time.record(request_start.elapsed().map_or(0.0, |d| d.as_secs_f64()), &labels);
                                response
                            })
                        }))
                    }
                });
                if let Err(e) = Server::bind(&private_http_socket).serve(http_service).await {
                    warn!("Problem starting HTTP interface {:?}", e);
                }
            };
        }
    });

    tasks.push(si_private_interface);
    tasks
}

fn start_pubsub_service(swir_config: &Swir, mmc: MessagingMemoryChannels, metric_registry: Arc<MetricRegistry>) -> Vec<tokio::task::JoinHandle<()>> {
    let mut tasks = vec![];
    let config = swir_config.pubsub.clone();
    let messaging = tokio::spawn(async move {
        messaging_handlers::configure_broker(config, mmc, metric_registry).await;
    });
    tasks.push(messaging);
    tasks
}

fn start_persistence_service(swir_config: &Swir, pmc: PersistenceMemoryChannels, metric_registry: Arc<MetricRegistry>) -> Vec<tokio::task::JoinHandle<()>> {
    let mut tasks = vec![];
    let config = swir_config.clone();
    let pmc = pmc.from_client_to_persistence_receivers_map;
    let persistence = tokio::spawn(async move {
        persistence_handlers::configure_stores(config.stores, pmc, metric_registry).await;
    });
    tasks.push(persistence);
    tasks
}

fn start_rest_client_service(_: &Swir, mc: &MemoryChannels, metric_registry: Arc<metric_utils::MetricRegistry>) -> Vec<tokio::task::JoinHandle<()>> {
    let mut tasks = vec![];
    let mmc = &mc.messaging_memory_channels;
    let to_client_receiver_for_rest = mmc.to_client_receiver_for_rest.clone();
    let http_client = tokio::spawn(async move { client_handler(to_client_receiver_for_rest.clone(), metric_registry).await });
    tasks.push(http_client);
    tasks
}
