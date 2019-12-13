use std::borrow::BorrowMut;
use std::sync::Arc;

use futures::channel::oneshot;
use futures::lock::Mutex;
use futures::stream::StreamExt;
use hyper::StatusCode;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::mpsc;
use tonic::{IntoRequest, Response, Status};

use crate::utils::structs::{EndpointDesc, Job, MessagingResult, MessagingToRestContext, RestToMessagingContext};

pub mod client_api {
    tonic::include_proto!("swir");
}

#[derive(Debug)]
pub struct SwirAPI {
    pub tx: Sender<crate::utils::structs::RestToMessagingContext>,
    pub rx: Arc<Mutex<Receiver<crate::utils::structs::MessagingToRestContext>>>,
}

#[tonic::async_trait]
impl client_api::clientapi_server::ClientApi for SwirAPI {
    async fn publish(
        &self,
        request: tonic::Request<client_api::PublishRequest>, // Accept request of type HelloRequest
    ) -> Result<tonic::Response<client_api::PublishResponse>, tonic::Status> { // Return an instance of type HelloReply
        println!("Got a request: {:?}", request);
        let p = crate::utils::structs::PublishRequest { payload: request.into_inner().payload, url: "".to_string() };
        debug!("{:?}", p);
        let (local_tx, local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
        let job = RestToMessagingContext { job: Job::Publish(p), sender: local_tx };
        let mut tx = self.tx.clone();
        if let Err(e) = tx.try_send(job) {
            warn!("Channel is dead {:?}", e);
        }

        debug!("Waiting for broker");
        let response_from_broker: Result<MessagingResult, oneshot::Canceled> = local_rx.await;
        debug!("Got result {:?}", response_from_broker);
        let mut msg = String::new();
        if let Ok(res) = response_from_broker {
            msg.push_str(res.result.as_str());
        } else {
            msg.push_str(StatusCode::INTERNAL_SERVER_ERROR.as_str());
        }
        let reply = client_api::PublishResponse {
            status: format!("{}", msg).into(), // We must use .into_inner() as the fields of gRPC requests and responses are private
        };
        Ok(tonic::Response::new(reply)) // Send back our formatted greeting
    }

    type SubscribeStream = tokio::sync::mpsc::Receiver<Result<client_api::SubscribeResponse, Status>>;

    async fn subscribe(
        &self,
        request: tonic::Request<client_api::SubscribeRequest>,
    ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
        let topic = request.into_inner().topic;
        println!("Topic = {:?}", topic);

        let sr = crate::utils::structs::SubscribeRequest {
            endpoint: EndpointDesc { url: "".to_string() }
        };

        let (local_tx, local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
        let job = RestToMessagingContext { job: Job::Subscribe(sr), sender: local_tx };
        info!("About to send to messaging processor");
        let mut tx = self.tx.clone();
        tx.try_send(job).unwrap();

        info!("Waiting for response from kafka");
        local_rx.await.unwrap();

        let mut loc_rx = self.rx.clone();
        let (mut tx, rx) = tokio::sync::mpsc::channel(4);
        tokio::spawn(async move {
            let mut loc_rx = loc_rx.lock().await;
            while let Some(messaging_context) = loc_rx.next().await {
                let s = client_api::SubscribeResponse {
                    payload: messaging_context.payload
                };
                let r = tx.send(Ok(s)).await;
                if let Err(e) = r {
                    error!("{:?}", e);
                }
            }
        });
        Ok(Response::new(rx))
    }
}
