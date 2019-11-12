use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishRequest {
    pub(crate) payload: String,
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
pub struct KafkaResult {
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
pub struct InternalMessage {
    pub job: Job,
    pub sender: Sender<KafkaResult>,
}
