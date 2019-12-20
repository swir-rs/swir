use std::collections::HashMap;
use std::sync::Arc;

use futures::channel::oneshot;
use futures::lock::Mutex;
use futures::stream::StreamExt;
use hyper::StatusCode;
use tokio::sync::mpsc;
use tonic::{Response, Status};

use crate::utils::structs::CustomerInterfaceType::GRPC;
use crate::utils::structs::{EndpointDesc, Job, MessagingResult, RestToMessagingContext};

pub mod client_api {
    tonic::include_proto!("swir");
}

#[derive(Debug)]
pub struct SwirAPI {
    pub from_client_to_backend_channel_sender: Box<HashMap<String, Box<mpsc::Sender<RestToMessagingContext>>>>,
    pub to_client_receiver: Arc<Mutex<mpsc::Receiver<crate::utils::structs::MessagingToRestContext>>>,
}

impl SwirAPI {
    fn find_channel(&self, topic_name: &String) -> Option<&Box<mpsc::Sender<RestToMessagingContext>>> {
        self.from_client_to_backend_channel_sender.get(topic_name)
    }
}

#[tonic::async_trait]
impl client_api::clientapi_server::ClientApi for SwirAPI {
    async fn publish(
        &self,
        request: tonic::Request<client_api::PublishRequest>, // Accept request of type HelloRequest
    ) -> Result<tonic::Response<client_api::PublishResponse>, tonic::Status> {
        // Return an instance of type HelloReply
        println!("Got a request: {:?}", request);
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
        println!("Topic = {:?}", topic);

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

        info!("Waiting for response from broker");
        local_rx.await.unwrap();

        let loc_rx = self.to_client_receiver.clone();
        let (mut tx, rx) = tokio::sync::mpsc::channel(4);
        tokio::spawn(async move {
            let mut loc_rx = loc_rx.lock().await;
            while let Some(messaging_context) = loc_rx.next().await {
                let s = client_api::SubscribeResponse { payload: messaging_context.payload };
                let r = tx.send(Ok(s)).await;
                if let Err(e) = r {
                    error!("{:?}", e);
                }
            }
        });
        Ok(Response::new(rx))
    }
}
