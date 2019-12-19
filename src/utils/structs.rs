use futures::channel::oneshot::Sender;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum CustomerInterfaceType {
    REST,
    GRPC,
}

impl CustomerInterfaceType {
    pub fn from_str(s: &str) -> Result<CustomerInterfaceType, ()> {
        match s {
            "REST" => Ok(CustomerInterfaceType::REST),
            "GRPC" => Ok(CustomerInterfaceType::GRPC),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishRequest {
    pub(crate) payload: Vec<u8>,
    pub(crate) client_topic: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EndpointDesc {
    pub(crate) url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClientSubscribeRequest {
    pub(crate) endpoint: EndpointDesc,
    pub(crate) client_topic: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubscribeRequest {
    pub(crate) endpoint: EndpointDesc,
    pub(crate) client_topic: String,
    pub(crate) client_interface_type: CustomerInterfaceType,
}

#[derive(Debug)]
pub struct MessagingResult {
    pub(crate) status: u32,
    pub(crate) result: String,
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
    pub uri: String,
}
