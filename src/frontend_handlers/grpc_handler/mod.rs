use hyper::StatusCode;

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::swir_common;
use crate::swir_grpc_api;
use crate::utils::config::ClientConfig;
use crate::utils::{metric_utils, metric_utils::MeteredClientService, structs::*};
use tracing::Span;
use tracing_futures::Instrument;

use crate::utils::tracing_utils;
use swir_grpc_api::notification_api_client::NotificationApiClient;
use tokio::time::Duration;
use tonic::transport::Endpoint;
use tonic::Request;

impl fmt::Display for swir_grpc_api::SubscribeRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SubscribeRequest {{ correlation_id:{}, topic:{}}}", &self.correlation_id, &self.topic)
    }
}

impl fmt::Display for swir_grpc_api::UnsubscribeRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UnsubscribeRequest {{ correlation_id:{}, topic:{}}}", &self.correlation_id, &self.topic)
    }
}

impl fmt::Display for swir_grpc_api::SubscribeNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SubscribeNotification {{ correlation_id:{}, payload:{}}}",
            &self.correlation_id,
            String::from_utf8_lossy(&self.payload)
        )
    }
}

impl fmt::Display for swir_grpc_api::PublishRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PublishRequest {{ correlation_id:{}, topic: {}, payload:{} }}",
            &self.correlation_id,
            &self.topic,
            String::from_utf8_lossy(&self.payload)
        )
    }
}

impl fmt::Display for swir_grpc_api::PublishResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublishResponse {{ correlation_id:{}, status: {} }}", &self.correlation_id, &self.status)
    }
}

impl fmt::Display for swir_grpc_api::StoreResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StoreResponse {{ correlation_id:{}, status: {} }}", &self.correlation_id, &self.status)
    }
}

impl fmt::Display for swir_grpc_api::StoreRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StoreRequest {{ correlation_id:{}, key:{}}}", &self.correlation_id, &self.key)
    }
}

impl fmt::Display for swir_grpc_api::RetrieveResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RetrieveResponse {{ correlation_id:{}, key:{}}}", &self.correlation_id, &self.key)
    }
}

impl fmt::Display for swir_grpc_api::RetrieveRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RetrieveRequest {{ correlation_id:{}, key: {} }}", &self.correlation_id, &self.key)
    }
}

impl fmt::Display for swir_grpc_api::DeleteResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeleteResponse {{ correlation_id:{}, key:{}}}", &self.correlation_id, &self.key)
    }
}

impl fmt::Display for swir_grpc_api::DeleteRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeleteRequest {{ correlation_id:{}, key: {} }}", &self.correlation_id, &self.key)
    }
}

#[derive(Debug)]
pub struct SwirPubSubApi {
    pub from_client_to_backend_channel_sender: HashMap<String, mpsc::Sender<RestToMessagingContext>>,
    pub to_client_sender: mpsc::Sender<BackendToRestContext>,
    pub metric_registry: Arc<metric_utils::MetricRegistry>,
}

impl SwirPubSubApi {
    fn find_channel(&self, topic_name: &str) -> Option<&mpsc::Sender<RestToMessagingContext>> {
        self.from_client_to_backend_channel_sender.get(topic_name)
    }

    pub fn new(
        from_client_to_backend_channel_sender: HashMap<String, mpsc::Sender<RestToMessagingContext>>,
        to_client_sender: mpsc::Sender<BackendToRestContext>,
        metric_registry: Arc<metric_utils::MetricRegistry>,
    ) -> SwirPubSubApi {
        SwirPubSubApi {
            from_client_to_backend_channel_sender,
            to_client_sender,
            metric_registry,
        }
    }
    async fn send_to_backend(&self, tx: &mut mpsc::Sender<RestToMessagingContext>, local_rx: oneshot::Receiver<MessagingResult>, ctx: RestToMessagingContext) -> String {
        if let Err(e) = tx.try_send(ctx) {
            warn!("Channel is dead {:?}", e);
        }

        let response_from_broker = local_rx.await;
        let mut msg = String::new();

        if let Ok(res) = response_from_broker {
            msg.push_str(&res.status.to_string());
        } else {
            msg.push_str(StatusCode::INTERNAL_SERVER_ERROR.as_str());
        }

        msg
    }

    async fn subscribe_unsubscribe_processor(&self, subscribe: bool, request: crate::utils::structs::SubscribeRequest, tx: &mut mpsc::Sender<RestToMessagingContext>) -> String {
        debug!("{}", request);
        let (local_tx, local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
        let ctx = if subscribe {
            RestToMessagingContext {
                job: Job::Subscribe(request),
                sender: local_tx,
                span: Span::current(),
            }
        } else {
            RestToMessagingContext {
                job: Job::Unsubscribe(request),
                sender: local_tx,
                span: Span::current(),
            }
        };
        self.send_to_backend(tx, local_rx, ctx).await
    }
}

#[tonic::async_trait]
impl swir_grpc_api::pub_sub_api_server::PubSubApi for SwirPubSubApi {
    async fn publish(&self, request: tonic::Request<swir_grpc_api::PublishRequest>) -> Result<tonic::Response<swir_grpc_api::PublishResponse>, tonic::Status> {
        let request = request.into_inner();
        info!("Publish {}", request);
        if let Some(tx) = self.find_channel(&request.topic) {
            let p = crate::utils::structs::PublishRequest {
                correlation_id: request.correlation_id.clone(),
                payload: request.payload,
                client_topic: request.topic.clone(),
            };
            debug!("{}", p);
            let (local_tx, local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
            let ctx = RestToMessagingContext {
                job: Job::Publish(p),
                sender: local_tx,
                span: Span::current(),
            };
            let mut tx = tx.clone();
            let msg = self.send_to_backend(&mut tx, local_rx, ctx).await;
            let reply = swir_grpc_api::PublishResponse {
                correlation_id: request.correlation_id,
                status: msg,
            };

            Ok(tonic::Response::new(reply))
        } else {
            Err(tonic::Status::invalid_argument("Invalid topic"))
        }
    }

    async fn subscribe(&self, request: tonic::Request<swir_grpc_api::SubscribeRequest>) -> Result<tonic::Response<swir_grpc_api::SubscribeResponse>, tonic::Status> {
        let request = request.into_inner();

        info!("Subscribe {}", request);

        if let Some(tx) = self.find_channel(&request.topic) {
            let p = crate::utils::structs::SubscribeRequest {
                correlation_id: request.correlation_id.clone(),
                client_interface_type: CustomerInterfaceType::GRPC,
                client_topic: request.topic,
                endpoint: EndpointDesc {
                    url: String::new(),
                    client_id: request.client_id,
                },
                tx: Box::new(self.to_client_sender.clone()),
            };
            let mut tx = tx.clone();
            let msg = self.subscribe_unsubscribe_processor(true, p, &mut tx).await;

            Ok(tonic::Response::new(swir_grpc_api::SubscribeResponse {
                correlation_id: request.correlation_id.clone(),
                status: msg,
            }))
        } else {
            Err(tonic::Status::invalid_argument("Invalid topic"))
        }
    }

    async fn unsubscribe(&self, request: tonic::Request<swir_grpc_api::UnsubscribeRequest>) -> Result<tonic::Response<swir_grpc_api::UnsubscribeResponse>, tonic::Status> {
        let request = request.into_inner();

        info!("Unsubscribe {}", request);

        if let Some(tx) = self.find_channel(&request.topic) {
            let p = crate::utils::structs::SubscribeRequest {
                correlation_id: request.correlation_id.clone(),
                client_interface_type: CustomerInterfaceType::GRPC,
                client_topic: request.topic,
                endpoint: EndpointDesc {
                    url: String::new(),
                    client_id: request.client_id,
                },
                tx: Box::new(self.to_client_sender.clone()),
            };
            let mut tx = tx.clone();
            let msg = self.subscribe_unsubscribe_processor(false, p, &mut tx).await;

            Ok(tonic::Response::new(swir_grpc_api::UnsubscribeResponse {
                correlation_id: request.correlation_id.clone(),
                status: msg,
            }))
        } else {
            Err(tonic::Status::invalid_argument("Invalid topic"))
        }
    }
}

#[derive(Debug)]
pub struct SwirPersistenceApi {
    pub from_client_to_backend_channel_sender: HashMap<String, mpsc::Sender<RestToPersistenceContext>>,
    pub metric_registry: Arc<metric_utils::MetricRegistry>,
}

impl SwirPersistenceApi {
    fn find_channel(&self, topic_name: &str) -> Option<&mpsc::Sender<RestToPersistenceContext>> {
        self.from_client_to_backend_channel_sender.get(topic_name)
    }

    pub fn new(from_client_to_backend_channel_sender: HashMap<String, mpsc::Sender<RestToPersistenceContext>>, metric_registry: Arc<metric_utils::MetricRegistry>) -> SwirPersistenceApi {
        SwirPersistenceApi {
            from_client_to_backend_channel_sender,
            metric_registry,
        }
    }
}

#[tonic::async_trait]
impl swir_grpc_api::persistence_api_server::PersistenceApi for SwirPersistenceApi {
    async fn store(&self, request: tonic::Request<swir_grpc_api::StoreRequest>) -> Result<tonic::Response<swir_grpc_api::StoreResponse>, tonic::Status> {
        let request = request.into_inner();

        info!("Store {}", request);
        if let Some(tx) = self.find_channel(&request.database_name) {
            let p = crate::utils::structs::StoreRequest {
                correlation_id: request.correlation_id.clone(),
                payload: request.payload,
                key: request.key.clone(),
                table_name: request.database_name.clone(),
            };
            debug!("{}", p);
            let (local_tx, local_rx): (oneshot::Sender<PersistenceResult>, oneshot::Receiver<PersistenceResult>) = oneshot::channel();
            let ctx = RestToPersistenceContext {
                job: PersistenceJobType::Store(p),
                sender: local_tx,
                span: Span::current(),
            };

            let tx = tx.clone();
            if let Err(e) = tx.try_send(ctx) {
                warn!("Channel is dead {:?}", e);
            }

            let response_from_storage = local_rx.await;
            let mut msg = String::new();

            if let Ok(res) = response_from_storage {
                msg.push_str(&res.status.to_string());
            } else {
                msg.push_str(StatusCode::INTERNAL_SERVER_ERROR.as_str());
            }
            let reply = swir_grpc_api::StoreResponse {
                correlation_id: request.correlation_id,
                database_name: request.database_name,
                key: request.key,
                status: msg,
            };

            Ok(tonic::Response::new(reply)) // Send back our formatted greeting
        } else {
            Err(tonic::Status::invalid_argument(format!("Invalid client database name {}", request.database_name)))
        }
    }
    async fn retrieve(&self, request: tonic::Request<swir_grpc_api::RetrieveRequest>) -> Result<tonic::Response<swir_grpc_api::RetrieveResponse>, tonic::Status> {
        let request = request.into_inner();

        info!("Retrieve {}", request);
        if let Some(tx) = self.find_channel(&request.database_name) {
            let p = crate::utils::structs::RetrieveRequest {
                table_name: request.database_name.clone(),
                correlation_id: request.correlation_id.clone(),
                key: request.key.clone(),
            };
            debug!("{}", p);
            let (local_tx, local_rx): (oneshot::Sender<PersistenceResult>, oneshot::Receiver<PersistenceResult>) = oneshot::channel();
            let ctx = RestToPersistenceContext {
                job: PersistenceJobType::Retrieve(p),
                sender: local_tx,
                span: Span::current(),
            };

            let tx = tx.clone();
            if let Err(e) = tx.try_send(ctx) {
                warn!("Channel is dead {:?}", e);
            }

            let response_from_storage = local_rx.await;

            let reply = if let Ok(res) = response_from_storage {
                let mut msg = String::new();
                msg.push_str(&res.status.to_string());
                swir_grpc_api::RetrieveResponse {
                    correlation_id: request.correlation_id,
                    database_name: request.database_name,
                    key: request.key,
                    payload: res.payload,
                    status: msg,
                }
            } else {
                let mut msg = String::new();

                msg.push_str(StatusCode::INTERNAL_SERVER_ERROR.as_str());
                swir_grpc_api::RetrieveResponse {
                    correlation_id: request.correlation_id,
                    database_name: request.database_name,
                    key: request.key,
                    payload: vec![],
                    status: msg,
                }
            };
            Ok(tonic::Response::new(reply)) // Send back our formatted greeting
        } else {
            Err(tonic::Status::invalid_argument(format!("Invalid client database name {}", request.database_name)))
        }
    }

    async fn delete(&self, request: tonic::Request<swir_grpc_api::DeleteRequest>) -> Result<tonic::Response<swir_grpc_api::DeleteResponse>, tonic::Status> {
        let request = request.into_inner();

        info!("Delete {}", request);
        if let Some(tx) = self.find_channel(&request.database_name) {
            let p = crate::utils::structs::DeleteRequest {
                table_name: request.database_name.clone(),
                correlation_id: request.correlation_id.clone(),
                key: request.key.clone(),
            };
            debug!("{}", p);
            let (local_tx, local_rx): (oneshot::Sender<PersistenceResult>, oneshot::Receiver<PersistenceResult>) = oneshot::channel();
            let ctx = RestToPersistenceContext {
                job: PersistenceJobType::Delete(p),
                sender: local_tx,
                span: Span::current(),
            };

            let tx = tx.clone();
            if let Err(e) = tx.try_send(ctx) {
                warn!("Channel is dead {:?}", e);
            }

            let response_from_storage = local_rx.await;

            let reply = if let Ok(res) = response_from_storage {
                let mut msg = String::new();
                msg.push_str(&res.status.to_string());
                swir_grpc_api::DeleteResponse {
                    correlation_id: request.correlation_id,
                    database_name: request.database_name,
                    key: request.key,
                    payload: res.payload,
                    status: msg,
                }
            } else {
                let mut msg = String::new();
                msg.push_str(StatusCode::INTERNAL_SERVER_ERROR.as_str());
                swir_grpc_api::DeleteResponse {
                    correlation_id: request.correlation_id,
                    database_name: request.database_name,
                    key: request.key,
                    payload: vec![],
                    status: msg,
                }
            };
            Ok(tonic::Response::new(reply)) // Send back our formatted greeting
        } else {
            Err(tonic::Status::invalid_argument(format!("Invalid client database name {}", request.database_name)))
        }
    }
}

#[derive(Debug)]
pub struct SwirServiceInvocationApi {
    pub from_client_to_si_sender: mpsc::Sender<RestToSIContext>,
    pub metric_registry: Arc<metric_utils::MetricRegistry>,
}

impl SwirServiceInvocationApi {
    pub fn new(from_client_to_si_sender: mpsc::Sender<RestToSIContext>, metric_registry: Arc<metric_utils::MetricRegistry>) -> Self {
        SwirServiceInvocationApi {
            from_client_to_si_sender,
            metric_registry,
        }
    }
}

#[tonic::async_trait]
impl swir_grpc_api::service_invocation_api_server::ServiceInvocationApi for SwirServiceInvocationApi {
    async fn invoke(&self, request: tonic::Request<swir_common::InvokeRequest>) -> Result<tonic::Response<swir_common::InvokeResponse>, tonic::Status> {
        let req = request.into_inner();

        info!("Invoke {}", &req);
        let correlation_id = req.correlation_id.clone();
        let service_name = req.service_name.clone();

        if !validate_method(req.method) {
            return Err(tonic::Status::invalid_argument("Unsupported method"));
        }

        let job = SIJobType::PublicInvokeGrpc { req };

        let (local_sender, local_rx): (oneshot::Sender<SIResult>, oneshot::Receiver<SIResult>) = oneshot::channel();

        let ctx = RestToSIContext {
            job,
            sender: local_sender,
            span: Span::current(),
        };
        let sender = self.from_client_to_si_sender.clone();
        let res = sender.try_send(ctx);
        if let Err(e) = res {
            warn!("Channel is dead {:?}", e);

            Err(tonic::Status::internal("Internal error"))
        } else {
            let response_from_service = local_rx.await;

            if let Ok(res) = response_from_service {
                debug!("Got result from internal {}", res);
                if let Some(si_response) = res.response {
                    Ok(tonic::Response::new(si_response))
                } else {
                    Ok(tonic::Response::new(swir_common::InvokeResponse {
                        correlation_id,
                        service_name,
                        result: Some(swir_common::InvokeResult {
                            status: swir_common::InvokeStatus::Error as i32,
                            msg: res.status.to_string(),
                        }),
                        ..Default::default()
                    }))
                }
            } else {
                Err(tonic::Status::internal("Internal error : canceled"))
            }
        }
    }
}

async fn send_request(mut client: NotificationApiClient<MeteredClientService>, ctx: BackendToRestContext) {
    let sreq = swir_grpc_api::SubscribeNotification {
        correlation_id: ctx.correlation_id,
        payload: ctx.request_params.payload,
    };

    let mut req = Request::new(sreq.clone());

    //let mut labels = metric_registry.grpc.labels.clone();
    // labels.push(KeyValue::new("interface", "subscription_notification"));
    // metric_registry.grpc.outgoing_counters.request_counter.add(1, &labels);

    if let Some((trace_header_name, trace_header)) = tracing_utils::get_grpc_tracing_header() {
        req.metadata_mut().insert(trace_header_name, trace_header);
    };

    match client.subscription_notification(req).await {
        Err(e) => warn!("Unable to send {} {:?}", sreq, e),
        Ok(resp) => info!("Notify {:?}", resp),
    }
}

pub async fn client_handler(client_config: ClientConfig, rx: Arc<Mutex<mpsc::Receiver<BackendToRestContext>>>, metric_registry: Arc<metric_utils::MetricRegistry>) {
    if let Some(port) = client_config.grpc_port {
        loop {
            let endpoint = Endpoint::from_shared(format! {"http://{}:{}", client_config.ip, port})
                .unwrap()
                .timeout(Duration::from_secs(2))
                .concurrency_limit(256)
                .connect()
                .await;
            if let Ok(channel) = endpoint {
                let metered_client = MeteredClientService {
                    inner: channel,
                    labels: metric_registry.grpc.labels.clone(),
                    counters: metric_registry.grpc.outgoing_counters.clone(),
                    histograms: metric_registry.grpc.outgoing_histograms.clone(),
                };

                //		let client = NotificationApiClient::new(channel);
                let client = NotificationApiClient::new(metered_client);

                info!("GRPC client created");
                let mut rx = rx.lock().await;
                while let Some(ctx) = rx.recv().await {
                    let client = client.clone();
                    let parent_span = ctx.span.clone();                   
                    let span = info_span!(parent: parent_span, "CLIENT_GRPC_OUTGOING");
                    tokio::spawn(async move { send_request(client, ctx).instrument(span).await });
                }
            } else {
                warn!("Can't create a GRPC channel {:?} {:?}", client_config, endpoint);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    } else {
        warn!("No GRPC port set. Client will not get any notifications");
        let mut rx = rx.lock().await;
        while let Some(ctx) = rx.recv().await {
            debug!("Discarding {}", ctx.correlation_id);
        }
    }
}
