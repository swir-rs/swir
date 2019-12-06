use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishRequest {
    pub(crate) payload: Vec<u8>,
    pub(crate) url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EndpointDesc {
    pub(crate) url: String,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct SubscribeRequest {
    pub(crate) endpoint: EndpointDesc
}


#[derive(Debug)]
pub struct MessagingResult {
    pub(crate) status: u32,
    pub(crate) result: String
}

#[derive(Debug)]
pub enum Job {
    Subscribe(SubscribeRequest),
    Publish(PublishRequest),
}

#[derive(Debug)]
pub struct CustomContext;

#[derive(Debug)]
pub struct RestToMessagingContext {
    pub job: Job,
    pub sender: Sender<MessagingResult>,
}

#[derive(Debug)]
pub struct MessagingToRestContext {
    pub sender: Sender<MessagingResult>,
    pub payload: Vec<u8>,
    pub uri: String
}
