pub mod si_http_handler;

use crate::service_discovery::ServiceDiscovery;
use crate::swir_common;
use crate::swir_grpc_internal_api;
use crate::utils::{config::Services, structs::*, tracing_utils};

use std::{collections::HashMap, fs, sync::Arc};
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    time::timeout,
};

use tonic::{
    metadata::AsciiMetadataValue,
    transport::{Certificate, Channel, ClientTlsConfig, Endpoint, Identity},
    Request,
};
use tower::discover::Change;

use tracing::{info_span, Span};
use tracing_futures::Instrument;

type GrpcClient = swir_grpc_internal_api::service_invocation_discovery_api_client::ServiceInvocationDiscoveryApiClient<Channel>;
type GrpcClients = HashMap<String, ClientHolder<String>>;

fn map_client_to_backend_status_calls(ccsc: ClientCallStatusCodes) -> (BackendStatusCodes, swir_common::InvokeResult) {
    match ccsc {
        ClientCallStatusCodes::Ok(msg) => (
            BackendStatusCodes::Ok(msg.clone()),
            swir_common::InvokeResult {
                status: swir_common::InvokeStatus::Ok as i32,
                msg,
            },
        ),
        ClientCallStatusCodes::Error(msg) => (
            BackendStatusCodes::Error(msg.clone()),
            swir_common::InvokeResult {
                status: swir_common::InvokeStatus::Error as i32,
                msg,
            },
        ),
    }
}

async fn invoke_handler(grpc_clients: Arc<Mutex<GrpcClients>>, sender: oneshot::Sender<SIResult>, req: swir_common::InvokeRequest) {
    let correlation_id = req.correlation_id.clone();
    let service_name = req.service_name.clone();

    debug!("invoke_handler: {} {}", &correlation_id, &service_name);
    let mut grpc_clients = grpc_clients.lock().await;
    if let Some(client_holder) = grpc_clients.get_mut(&req.service_name) {
        let mut req = Request::new(req);
        req.metadata_mut().insert("x-correlation-id", AsciiMetadataValue::from_str(&correlation_id).unwrap());
        let hostname = hostname::get().unwrap();
        req.metadata_mut().insert("origin", AsciiMetadataValue::from_str(hostname.to_str().unwrap()).unwrap());
        if let Some((trace_header_name, trace_header)) = tracing_utils::get_grpc_tracing_header() {
            req.metadata_mut().insert(trace_header_name, trace_header);
        };

        let resp = client_holder.client.invoke(req);
        let maybe_timeout = timeout(tokio::time::Duration::from_secs(2), resp).await;
        trace!("public_invoke_handler: got response on internal {:?}", maybe_timeout);
        if let Ok(grpc_result) = maybe_timeout {
            if let Ok(invoke_result) = grpc_result {
                let result = invoke_result.into_inner();
                let _res = sender.send(SIResult {
                    correlation_id,
                    status: BackendStatusCodes::Ok("Service call ok".to_string()),
                    response: Some(result),
                });
            } else {
                let _res = sender.send(SIResult {
                    correlation_id,
                    status: BackendStatusCodes::Error(grpc_result.unwrap_err().to_string()),
                    response: None,
                });
            }
        } else {
            let _res = sender.send(SIResult {
                correlation_id,
                status: BackendStatusCodes::Error(maybe_timeout.unwrap_err().to_string()),
                response: None,
            });
        }
    } else {
        let _res = sender.send(SIResult {
            correlation_id,
            status: BackendStatusCodes::NoService(format!("Service {} has not been resolved", service_name)),
            response: None,
        });
    }
}

struct ClientHolder<T: PartialEq> {
    key: T,
    client: Box<GrpcClient>,
    endpoint_manager: SimpleEndpointManager<usize>,
}

impl<T: PartialEq> PartialEq for ClientHolder<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T: Eq> Eq for ClientHolder<T> {}

#[derive(Clone)]
struct SimpleEndpointManager<K: Copy + std::ops::AddAssign + std::convert::From<u16>> {
    key_counter: K,
    managed_endpoints: Box<HashMap<String, K>>,
    rx: tokio::sync::mpsc::Sender<Change<K, Endpoint>>,
}

impl<K: Copy + std::ops::AddAssign + std::convert::From<u16>> SimpleEndpointManager<K> {
    fn new(rx: tokio::sync::mpsc::Sender<Change<K, Endpoint>>) -> Self {
        SimpleEndpointManager {
            key_counter: K::from(1u16),
            managed_endpoints: Box::new(HashMap::new()),
            rx,
        }
    }

    async fn add(&mut self, uri: &str, endpoint: Endpoint) {
        if let None = self.managed_endpoints.get(uri) {
            self.key_counter += K::from(1u16);
            if let Ok(_) = self.rx.send(Change::Insert(self.key_counter, endpoint)).await {
                self.managed_endpoints.insert(uri.to_string(), self.key_counter);
            }
        } else {
            debug!("Already exists {}", uri);
            return;
        };
    }

    #[allow(dead_code)]
    async fn remove(&mut self, uri: &str) {
        if let Some(key) = self.managed_endpoints.remove(uri) {
            let _res = self.rx.send(Change::Remove(key)).await;
        }
    }
}

pub struct ServiceInvocationService {
    grpc_clients: Arc<Mutex<GrpcClients>>,
}

impl ServiceInvocationService {
    pub fn new() -> Self {
        ServiceInvocationService {
            grpc_clients: Arc::new(Mutex::new(GrpcClients::new())),
        }
    }

    pub async fn start<T: ServiceDiscovery>(&self, services: Services, resolver: &T, receiver: Arc<Mutex<mpsc::Receiver<RestToSIContext>>>, http_sender: mpsc::Sender<BackendToRestContext>) {
        let services_to_resolve = services.resolve_services.clone();
        let services_to_announce = services.announce_services.clone();

        let (sender, mut resolve_receiver) = tokio::sync::mpsc::channel(10);

        let services_to_announce = services_to_announce.clone();
        for svc in services_to_announce.iter() {
            resolver.announce(&svc).await;
        }

        for svc in services_to_resolve.iter() {
            resolver.resolve(svc, sender.clone()).await;
        }

        let server_root_ca_cert = fs::read(services.tls_config.server_ca_cert).unwrap();
        let server_root_ca_cert = Certificate::from_pem(server_root_ca_cert.clone());
        let client_cert = fs::read(services.tls_config.client_cert.clone()).unwrap();
        let client_key = fs::read(services.tls_config.client_key.clone()).unwrap();
        let client_identity = Identity::from_pem(client_cert, client_key);
        let domain_name = services.tls_config.domain_name.clone();

        let grpc_clients = self.grpc_clients.clone();
        let mut tasks = vec![];
        let h2 = tokio::spawn(async move {
            while let Some(resolved_addr) = resolve_receiver.recv().await {
                let socket_addr = &resolved_addr.socket_addr;
                let service_name = &resolved_addr.service_name;
                let fqdn = &resolved_addr.fqdn;
                let port = &resolved_addr.port;
                debug!("resolver: service {:?}", resolved_addr);
                let uri = if let Some(fqdn) = fqdn {
                    format!("http://{}:{}", fqdn, port)
                } else if let Some(socket_addr) = socket_addr {
                    format!("http://{}", socket_addr)
                } else {
                    continue;
                };

                let mut grpc_clients = grpc_clients.lock().await;
                if let Ok(endpoint) = Endpoint::from_shared(uri.to_string()) {
                    let tls = ClientTlsConfig::new()
                        .domain_name(domain_name.clone())
                        .ca_certificate(server_root_ca_cert.clone())
                        .identity(client_identity.clone());

                    let endpoint = endpoint.tls_config(tls).unwrap();
                    let endpoint = endpoint.timeout(std::time::Duration::from_millis(500));
                    match grpc_clients.get_mut(service_name) {
                        Some(svc_holder) => {
                            let holder = &mut svc_holder.endpoint_manager;
                            holder.add(&uri, endpoint).await;
                        }
                        None => {
                            let (channel, tx) = Channel::balance_channel(10);
                            let manager = SimpleEndpointManager::new(tx);
                            let mut endpoint_manager = Box::new(manager.clone());
                            endpoint_manager.add(&uri, endpoint).await;
                            let client = GrpcClient::new(channel);

                            grpc_clients.insert(
                                service_name.clone(),
                                ClientHolder {
                                    key: uri,
                                    client: Box::new(client),
                                    endpoint_manager: manager,
                                },
                            );
                        }
                    }
                } else {
                    warn!("invalid uri {}", &uri);
                    continue;
                };
            }
            warn!("Channel closed");
        });

        tasks.push(h2);

        let client_endpoint_mapping = services.announce_services.clone();
        let mut receiver = receiver.lock().await;
        while let Some(ctx) = receiver.recv().await {
            let grpc_clients = self.grpc_clients.clone();
            let http_sender = http_sender.clone();
            let client_endpoint_mapping = client_endpoint_mapping.clone();
            let parent_span = ctx.span;
            let _s = parent_span.enter();
            let span = info_span!("service_invocation");
            let job = ctx.job;
            let sender = ctx.sender;
            tokio::spawn(
                async move {
                    let client_endpoint_mapping = client_endpoint_mapping.clone();

                    match job {
                        SIJobType::PublicInvokeGrpc { req } => {
                            invoke_handler(grpc_clients, sender, req).await;
                        }
                        SIJobType::PublicInvokeHttp { req } => {
                            invoke_handler(grpc_clients, sender, req).await;
                        }
                        SIJobType::InternalInvoke { req } => {
                            debug!("internal_invoke_handler: {}", req);
                            let (s, r) = oneshot::channel();
                            let correlation_id = req.correlation_id;
                            let service_name = req.service_name.clone();
                            let client_endpoint = client_endpoint_mapping
                                .iter()
                                .filter(|s| s.service_details.service_name == service_name)
                                .map(|s| s.client_url.clone())
                                .next();
                            if let Some(endpoint) = client_endpoint {
                                let method = swir_common::HttpMethod::from_i32(req.method).unwrap();
                                let mrc = BackendToRestContext {
                                    span: Span::current(),
                                    correlation_id: correlation_id.clone(),
                                    sender: Some(s),
                                    request_params: RESTRequestParams {
                                        payload: req.payload,
                                        headers: req.headers,
                                        method: method.to_string(),
                                        uri: format!("{}{}", endpoint, req.request_target),
                                    },
                                };

                                if let Err(mpsc::error::SendError(_)) = http_sender.send(mrc).await {
                                    warn!("Unable to send {} {}. Channel is closed", service_name, correlation_id);
                                    let _res = sender.send(SIResult {
                                        correlation_id,
                                        status: BackendStatusCodes::Error("Internal ereror".to_string()),
                                        response: None,
                                    });
                                } else if let Ok(response_from_client) = r.await {
                                    let (status, result) = map_client_to_backend_status_calls(response_from_client.status);
                                    let _res = sender.send(SIResult {
                                        correlation_id: correlation_id.clone(),
                                        status,
                                        response: Some(swir_common::InvokeResponse {
                                            correlation_id,
                                            service_name,
                                            result: Some(result),
                                            payload: response_from_client.response_params.payload.to_owned(),
                                            status_code: response_from_client.response_params.status_code as i32,
                                            headers: response_from_client.response_params.headers,
                                        }),
                                    });
                                } else {
                                    let _res = sender.send(SIResult {
                                        correlation_id,
                                        status: BackendStatusCodes::Error("Internal ereror".to_string()),
                                        response: None,
                                    });
                                }
                            } else {
                                let msg = format!("Can't find client url for service name {}", service_name);
                                warn!("{}", msg);
                                let _res = sender.send(SIResult {
                                    correlation_id,
                                    status: BackendStatusCodes::Error(msg.to_string()),
                                    response: None,
                                });
                            }
                        }
                    }
                }
                .instrument(span),
            );
        }

        futures::future::join_all(tasks).await;
    }
}
