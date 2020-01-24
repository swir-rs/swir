use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use futures::channel::oneshot;
use futures::lock::Mutex;
use futures::stream::StreamExt;
use hyper::StatusCode;
use tokio::sync::mpsc;
use tonic::{Response, Status};

use crate::utils::structs::{EndpointDesc, Job, MessagingResult, MessagingToRestContext, RestToMessagingContext};
use crate::utils::structs::CustomerInterfaceType::GRPC;

pub mod client_api {
    tonic::include_proto!("swir");
}

#[derive(Debug)]
pub struct SwirAPI {
    missed_messages: Arc<Mutex<Box<VecDeque<client_api::SubscribeResponse>>>>,
    pub from_client_to_backend_channel_sender: Box<HashMap<String, Box<mpsc::Sender<RestToMessagingContext>>>>,
    pub to_client_receiver: Arc<Mutex<mpsc::Receiver<crate::utils::structs::MessagingToRestContext>>>,
}

impl SwirAPI {
    fn find_channel(&self, topic_name: &String) -> Option<&Box<mpsc::Sender<RestToMessagingContext>>> {
        self.from_client_to_backend_channel_sender.get(topic_name)
    }

    pub fn new(
        from_client_to_backend_channel_sender: Box<HashMap<String, Box<mpsc::Sender<RestToMessagingContext>>>>,
        to_client_receiver: Arc<Mutex<mpsc::Receiver<MessagingToRestContext>>>,
    ) -> SwirAPI {
        let missed_messages = Arc::new(Mutex::new(Box::new(VecDeque::new())));
        SwirAPI {
            missed_messages,
            from_client_to_backend_channel_sender,
            to_client_receiver,
        }
    }
}

#[tonic::async_trait]
impl client_api::client_api_server::ClientApi for SwirAPI {
    async fn publish(
        &self,
        request: tonic::Request<client_api::PublishRequest>, // Accept request of type HelloRequest
    ) -> Result<tonic::Response<client_api::PublishResponse>, tonic::Status> {
        // Return an instance of type HelloReply
        debug!("Got a request: {:?}", request);
        let request = request.into_inner();

        if let Some(tx) = self.find_channel(&request.topic) {
            let p = crate::utils::structs::PublishRequest {
                payload: request.payload,
                client_topic: request.topic.clone(),
            };
            debug!("{:?}", p);
            let (local_tx, local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
            let job = RestToMessagingContext {
                job: Job::Publish(p),
                sender: local_tx,
            };

            let mut tx = tx.clone();
            if let Err(e) = tx.try_send(job) {
                warn!("Channel is dead {:?}", e);
            }

            debug!("Waiting for broker");
            let response_from_broker: Result<MessagingResult, oneshot::Canceled> = local_rx.await;
            debug!("Got result {:?}", response_from_broker);
            let mut msg = String::new();
            if let Ok(res) = response_from_broker {
                msg.push_str(&res.status.to_string());
            } else {
                msg.push_str(StatusCode::INTERNAL_SERVER_ERROR.as_str());
            }
            let reply = client_api::PublishResponse {
                status: format!("{}", msg).into(), // We must use .into_inner() as the fields of gRPC requests and responses are private
            };
            Ok(tonic::Response::new(reply)) // Send back our formatted greeting
        } else {
            Err(tonic::Status::invalid_argument("Invalid topic"))
        }
    }

    type SubscribeStream = tokio::sync::mpsc::Receiver<Result<client_api::SubscribeResponse, Status>>;

    async fn subscribe(&self, request: tonic::Request<client_api::SubscribeRequest>) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
        let topic = request.into_inner().topic;
        info!("Topic = {:?}", topic);

        let sr = crate::utils::structs::SubscribeRequest {
            endpoint: EndpointDesc { url: "".to_string() },
            client_topic: topic.clone(),
            client_interface_type: GRPC,
        };

        let (local_tx, local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
        let job = RestToMessagingContext {
            job: Job::Subscribe(sr),
            sender: local_tx,
        };
        info!("About to send to messaging processor");

        if let Some(tx) = self.find_channel(&topic) {
            let mut tx = tx.clone();
            if let Err(e) = tx.try_send(job) {
                warn!("Channel is dead {:?}", e);
            }
        } else {
            return Err(tonic::Status::invalid_argument("Invalid topic"));
        }

        let (mut tx, rx) = tokio::sync::mpsc::channel(10);
        let missed_messages = self.missed_messages.clone();
        let loc_rx = self.to_client_receiver.clone();

        tokio::spawn(async move {
            let mut missed_messages = missed_messages.lock().await;
            while let Some(s) = missed_messages.pop_front() {
                let r = tx.send(Ok(s.clone())).await; //.expect("I should not panic as I should not be here!");
                info!("Message sent from the queue {:?} {:?}", r, s);
                if let Err(_) = r {
                    info!("Message pushed front  {:?}", s);
                    missed_messages.push_front(s);
                    return;
                }
            }

            let loc_rx = loc_rx.try_lock();
            if loc_rx.is_some() {
                if let Some(mut lrx) = loc_rx {
                    info!("Lock acquired {:?}", lrx);
                    while let Some(messaging_context) = lrx.next().await {
                        let s = client_api::SubscribeResponse { payload: messaging_context.payload };
                        let r = tx.send(Ok(s.clone())).await; //.expect("I should not panic as I should not be here!");
                        if let Err(_) = r {
                            info!("Message pushed back  {:?}", s);
                            missed_messages.push_back(s);
                            return;
                        }
                    }
                }
            } else {
                warn!("Unable to lock the rx");
            }
        });

        Ok(Response::new(rx))
    }
}
