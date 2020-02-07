use std::fmt;
use std::cmp::Ordering;
use futures::channel::oneshot::Sender;
use serde::export::fmt::Error;
use serde::export::Formatter;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;


#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize,Ord,PartialOrd)]

pub enum CustomerInterfaceType {
    REST,
    GRPC,
}

impl CustomerInterfaceType {
    //    pub fn from_str(s: &str) -> Result<CustomerInterfaceType, ()> {
    //        match s {
    //            "REST" => Ok(CustomerInterfaceType::REST),
    //            "GRPC" => Ok(CustomerInterfaceType::GRPC),
    //            _ => Err(()),
    //        }
    //    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct PublishRequest {
    pub(crate) payload: Vec<u8>,
    pub(crate) client_topic: String,
}

impl fmt::Display for PublishRequest{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublishRequest {{ client_topic: {}, payload:{} }}", &self.client_topic, String::from_utf8_lossy(&self.payload))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone,Eq,PartialEq,Ord,PartialOrd)]
pub struct EndpointDesc {
    pub(crate) url: String,
    pub(crate) client_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClientSubscribeRequest {
    pub(crate) endpoint: EndpointDesc,
    pub(crate) client_topic: String,
}

#[derive(Clone,Debug)]
pub struct SubscribeRequest {
    pub(crate) endpoint: EndpointDesc,
    pub(crate) client_topic: String,
    pub(crate) client_interface_type: CustomerInterfaceType,
    pub(crate) tx: Box<mpsc::Sender<MessagingToRestContext>>
}

impl Eq for SubscribeRequest {
}


impl fmt::Display for SubscribeRequest{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SubscribeRequest {{ endpoint: {:?}, client_topic:{}, client_interface_type:{:?} }}", &self.endpoint, &self.client_topic, &self.client_interface_type)
    }
}


impl PartialEq for SubscribeRequest {
    fn eq(&self, other: &Self) -> bool {
        if self.client_topic != other.client_topic{
	    false
	}else if self.endpoint != other.endpoint{
	    false
	}else if self.client_interface_type != other.client_interface_type{
	    false
	}else{
	    true
	}	    		    	    
    }
}

impl Ord for SubscribeRequest {
    fn cmp(&self, other: &Self) -> Ordering {
	let cmp = self.client_topic.cmp(&other.client_topic);
	if cmp==Ordering::Equal{
	    let cmp = self.endpoint.cmp(&other.endpoint);
	    if cmp==Ordering::Equal{
		self.client_interface_type.cmp(&other.client_interface_type)
	    }else{
		cmp
	    }
	    
	}else{
	    cmp
	}	
    }
}

impl PartialOrd for SubscribeRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
	Some(self.cmp(&other))
    }
}
      




#[derive(Debug)]
pub enum BackendStatusCodes {
    Ok(String),
    Error(String),
    NoTopic(String),
}

impl fmt::Display for BackendStatusCodes {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            BackendStatusCodes::Ok(msg) => write!(f, "BackendStatusCodes::Ok {}", msg),
            BackendStatusCodes::Error(msg) => write!(f, "BackendStatusCodes::ERR {}", msg),
            BackendStatusCodes::NoTopic(msg) => write!(f, "BackendStatusCodes::NoTopic {}", msg),
        }
    }
}

#[derive(Debug)]
pub struct MessagingResult {
    pub(crate) status: BackendStatusCodes,
}

#[derive(Debug)]
pub enum Job {
    Subscribe(SubscribeRequest),
    Unsubscribe(SubscribeRequest),
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
