use base64;
use futures::StreamExt;
use hyper::StatusCode;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::{mpsc, oneshot, Mutex};
use tonic::{Response, Status};

use crate::utils::structs::CustomerInterfaceType::GRPC;
use crate::utils::structs::*;

use crate::swir_common;
use crate::swir_grpc_api;
use tracing::Span;
use tracing_futures::Instrument;

impl fmt::Display for swir_grpc_api::SubscribeRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SubscribeRequest {{ correlation_id:{}, topic:{}}}", &self.correlation_id, &self.topic)
    }
}

impl fmt::Display for swir_grpc_api::SubscribeResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SubscribeResponse {{ correlation_id:{}, payload:{}}}", &self.correlation_id, String::from_utf8_lossy(&self.payload))
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
    missed_messages: Arc<Mutex<Box<VecDeque<swir_grpc_api::SubscribeResponse>>>>,
    pub from_client_to_backend_channel_sender: HashMap<String, mpsc::Sender<RestToMessagingContext>>,
    pub to_client_receiver: Arc<Mutex<mpsc::Receiver<crate::utils::structs::BackendToRestContext>>>,
}

impl SwirPubSubApi {
    fn find_channel(&self, topic_name: &str) -> Option<&mpsc::Sender<RestToMessagingContext>> {
        self.from_client_to_backend_channel_sender.get(topic_name)
    }

    pub fn new(from_client_to_backend_channel_sender: HashMap<String, mpsc::Sender<RestToMessagingContext>>, to_client_receiver: Arc<Mutex<mpsc::Receiver<BackendToRestContext>>>) -> SwirPubSubApi {
        let missed_messages = Arc::new(Mutex::new(Box::new(VecDeque::new())));
        SwirPubSubApi {
            missed_messages,
            from_client_to_backend_channel_sender,
            to_client_receiver,
        }
    }
}

#[tonic::async_trait]
impl swir_grpc_api::pub_sub_api_server::PubSubApi for SwirPubSubApi {
    type PublishBiStreamStream = mpsc::Receiver<Result<swir_grpc_api::PublishResponse, Status>>;

    async fn publish_bi_stream(&self, request: tonic::Request<tonic::Streaming<swir_grpc_api::PublishRequest>>) -> Result<tonic::Response<Self::PublishBiStreamStream>, tonic::Status> {
        info!("Publish bidi stream");
        let stream = request.into_inner();
        let (mut tx, rx) = mpsc::channel(10);
        let (mut internal_tx, mut internal_rx): (mpsc::Sender<swir_grpc_api::PublishResponse>, mpsc::Receiver<swir_grpc_api::PublishResponse>) = mpsc::channel(1);

        let error1 = Arc::new(AtomicBool::new(false));
        let error2 = error1.clone();

        let channels = self.from_client_to_backend_channel_sender.clone();

        tokio::spawn(
            async move {
                let mut cond = false;
                let error = error1.clone();
                while !cond {
                    let response = internal_rx.next().await;
                    match response {
                        Some(response) => {
                            debug!("Got message {}", response);
                            let pr: swir_grpc_api::PublishResponse = response.clone();
                            let r = tx.send(Ok(pr.clone())).await; //.expect("I should not panic as I should not be here!");
                            if r.is_err() {
                                error.swap(true, Ordering::Relaxed);
                                info!("gRPC connection closed for message {}", pr);
                                break;
                            } else {
                                debug!("Message sent {}", pr);
                            }
                        }
                        None => {
                            info!("Internal channel closed");
                            error.swap(true, Ordering::Relaxed);
                            break;
                        }
                    }
                    cond = error.load(Ordering::Relaxed);
                }
                info!("publish_bi_strean sender 1 terminated");
            }
            .instrument(Span::current()),
        );

        tokio::spawn(
            async move {
                futures::pin_mut!(stream);
                let mut cond = false;
                let error = error2.clone();
                while !cond {
                    let error = error.clone();
                    let request = stream.next().await;
                    match request {
                        Some(request) => {
                            if let Ok(request) = request {
                                info!("Publish request {}", request);
                                let mut msg = String::new();

                                if let Some(tx) = channels.get(&request.topic) {
                                    let p = crate::utils::structs::PublishRequest {
                                        correlation_id: request.correlation_id.clone(),
                                        payload: request.payload,
                                        client_topic: request.topic.clone(),
                                    };

                                    let (local_tx, local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
                                    let ctx = RestToMessagingContext {
                                        job: Job::Publish(p),
                                        sender: local_tx,
                                        span: Span::current(),
                                    };

                                    let mut tx = tx.clone();
                                    if let Err(e) = tx.try_send(ctx) {
                                        warn!("Channel is dead {:?}", e);
                                    }

                                    let response_from_broker = local_rx.await;
                                    if let Ok(res) = response_from_broker {
                                        msg.push_str(&res.status.to_string());
                                    } else {
                                        msg.push_str("problem with backend");
                                    }
                                } else {
                                    msg.push_str("Invalid token");
                                }
                                let reply = swir_grpc_api::PublishResponse {
                                    correlation_id: request.correlation_id,
                                    status: msg.to_string(), // We must use .into_inner() as the fields of gRPC requests and responses are private
                                };
                                debug!("Sending internally  {}", reply);
                                let r = internal_tx.send(reply.clone()).await;

                                if r.is_err() {
                                    info!("Internal channel closed for message {}", reply);
                                    error.swap(true, Ordering::Relaxed);
                                    break;
                                }
                            }
                        }
                        None => {
                            debug!("End of stream");
                            error.swap(true, Ordering::Relaxed);
                        }
                    }
                    cond = error.load(Ordering::Relaxed);
                }
                info!("publish_bi_stream sender 2 terminated");
            }
            .instrument(Span::current()),
        );
        Ok(tonic::Response::new(rx))
    }

    async fn publish(
        &self,
        request: tonic::Request<swir_grpc_api::PublishRequest>, // Accept request of type HelloRequest
    ) -> Result<tonic::Response<swir_grpc_api::PublishResponse>, tonic::Status> {
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
            let reply = swir_grpc_api::PublishResponse {
                correlation_id: request.correlation_id,
                status: msg,
            };
            Ok(tonic::Response::new(reply)) // Send back our formatted greeting
        } else {
            Err(tonic::Status::invalid_argument("Invalid topic"))
        }
    }

    type SubscribeStream = mpsc::Receiver<Result<swir_grpc_api::SubscribeResponse, Status>>;

    async fn subscribe(&self, request: tonic::Request<swir_grpc_api::SubscribeRequest>) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
        let request = request.into_inner();
        info!("Subscribe {}", request);
        let topic = request.topic;
        let correlation_id = request.correlation_id;
        let (to_client_tx, mut to_client_rx) = mpsc::channel::<BackendToRestContext>(1000);
        let mut small_rng = SmallRng::from_entropy();
        let mut array: [u8; 32] = [0; 32];
        small_rng.fill(&mut array);
        let client_id = base64::encode(&array);
        let sr = crate::utils::structs::SubscribeRequest {
            correlation_id: correlation_id.clone(),
            endpoint: EndpointDesc { url: "".to_string(), client_id },
            client_topic: topic.clone(),
            client_interface_type: GRPC,
            tx: Box::new(to_client_tx),
        };

        let (local_tx, _local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
        let subscribe_ctx = RestToMessagingContext {
            job: Job::Subscribe(sr.clone()),
            sender: local_tx,
            span: Span::current(),
        };

        let mut txx;
        if let Some(tx) = self.find_channel(&topic) {
            txx = tx.clone();
            if let Err(e) = txx.try_send(subscribe_ctx) {
                warn!("Channel is dead {:?}", e);
                return Err(tonic::Status::internal("Internal error"));
            }
        } else {
            return Err(tonic::Status::invalid_argument("Invalid topic"));
        }

        let (mut tx, rx) = mpsc::channel(1000);
        tokio::spawn(async move {
            let mut msgs: i32 = 0;
            while let Some(ctx) = to_client_rx.recv().await {
                let s = swir_grpc_api::SubscribeResponse {
                    correlation_id: correlation_id.clone(),
                    payload: ctx.request_params.payload,
                };
                msgs += 1;
                debug!("Sending message {}", s);
                let r = tx.send(Ok(s.clone())).await; //.expect("I should not panic as I should not be here!");
                if r.is_err() {
                    info!("Message pushed back {}", s);
                    let (unsub_tx, _unsub_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
                    let unsubscribe_ctx = RestToMessagingContext {
                        job: Job::Unsubscribe(sr.clone()),
                        sender: unsub_tx,
                        span: Span::current(),
                    };

                    if let Err(e) = txx.try_send(unsubscribe_ctx) {
                        warn!("Channel is dead {:?}", e);
                    }
                    debug!("Messages processed in this session {}", msgs);
                }
            }
        });

        Ok(Response::new(rx))
    }
}

#[derive(Debug)]
pub struct SwirPersistenceApi {
    pub from_client_to_backend_channel_sender: HashMap<String, mpsc::Sender<RestToPersistenceContext>>,
}

impl SwirPersistenceApi {
    fn find_channel(&self, topic_name: &str) -> Option<&mpsc::Sender<RestToPersistenceContext>> {
        self.from_client_to_backend_channel_sender.get(topic_name)
    }

    pub fn new(from_client_to_backend_channel_sender: HashMap<String, mpsc::Sender<RestToPersistenceContext>>) -> SwirPersistenceApi {
        SwirPersistenceApi {
            from_client_to_backend_channel_sender,
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

            let mut tx = tx.clone();
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

            let mut tx = tx.clone();
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

            let mut tx = tx.clone();
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
}

impl SwirServiceInvocationApi {
    pub fn new(from_client_to_si_sender: mpsc::Sender<RestToSIContext>) -> Self {
        SwirServiceInvocationApi { from_client_to_si_sender }
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
        let mut sender = self.from_client_to_si_sender.clone();
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
