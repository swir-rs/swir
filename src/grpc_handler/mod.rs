use futures::channel::mpsc::Sender;
use futures::channel::oneshot;
use hyper::StatusCode;

use client_api::{
    PublishRequest,
    PublishResponse, server::{ClientApi, ClientApiServer},
};

use crate::utils::structs::{Job, MessagingResult, RestToMessagingContext};

pub mod client_api {
    tonic::include_proto!("swir");
}

#[derive(Debug)]
pub struct SwirAPI {
    pub tx: Sender<crate::utils::structs::RestToMessagingContext>
}

#[tonic::async_trait]
impl ClientApi for SwirAPI {
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
}