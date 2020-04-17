use std::fmt;
use std::cmp::Ordering;
use futures::channel::oneshot::Sender;
use serde::export::fmt::Error;
use serde::export::Formatter;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use custom_error::custom_error;

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize,Ord,PartialOrd)]

pub enum CustomerInterfaceType {
    REST,
    GRPC,
}




#[derive(Serialize, Deserialize, Debug)]
pub struct PublishRequest {
    pub(crate) correlation_id: String,
    pub(crate) payload: Vec<u8>,
    pub(crate) client_topic: String,
}

impl fmt::Display for PublishRequest{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublishRequest {{ correlation_id: {}, client_topic: {}, payload:{} }}", &self.correlation_id, &self.client_topic, String::from_utf8_lossy(&self.payload))
    }
}

pub trait PersistenceRequest{
    fn get_correlation_id(&self)->String;
    fn get_table_name(&self)->String;	
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StoreRequest {
    pub(crate) correlation_id: String,
    pub(crate) payload: Vec<u8>,
    pub(crate) key: String,
    pub(crate) table_name: String
}

impl PersistenceRequest for StoreRequest{
    fn get_correlation_id(&self)->String{
	self.correlation_id.clone()
    }
    fn get_table_name(&self)->String{
	self.table_name.clone()
    }    
}

impl fmt::Display for StoreRequest{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StoreRequest {{ correlation_id: {}, key: {}, payload:{} }}", &self.correlation_id, &self.key, String::from_utf8_lossy(&self.payload))
    }
}

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct RetrieveRequest {
    pub(crate) correlation_id: String,
    pub(crate) table_name: String,
    pub(crate) key: String
}

impl PersistenceRequest for RetrieveRequest{
    fn get_correlation_id(&self)->String{
	self.correlation_id.clone()
    }
    fn get_table_name(&self)->String{
	self.table_name.clone()
    }    
}

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct DeleteRequest {
    pub(crate) correlation_id: String,
    pub(crate) table_name: String,
    pub(crate) key: String
}

impl PersistenceRequest for DeleteRequest{
    fn get_correlation_id(&self)->String{
	self.correlation_id.clone()
    }
    fn get_table_name(&self)->String{
	self.table_name.clone()
    }    
}

impl fmt::Display for RetrieveRequest{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RetrieveRequest {{ correlation_id: {}, key: {} }}", &self.correlation_id, &self.key)
    }
}

impl fmt::Display for DeleteRequest{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeleteRequest {{ correlation_id: {}, key: {} }}", &self.correlation_id, &self.key)
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
    pub(crate) correlation_id: String,
    pub(crate) client_topic: String,
    pub(crate) client_interface_type: CustomerInterfaceType,
    pub(crate) tx: Box<mpsc::Sender<BackendToRestContext>>
}

impl Eq for SubscribeRequest {
}


impl fmt::Display for SubscribeRequest{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SubscribeRequest {{ correlation_id: {}, endpoint: {:?}, client_topic:{}, client_interface_type:{:?} }}", &self.correlation_id, &self.endpoint, &self.client_topic, &self.client_interface_type)
    }
}


impl PartialEq for SubscribeRequest {
    fn eq(&self, other: &Self) -> bool {
	let c1 = self.correlation_id == other.correlation_id;
	let c2 = self.client_topic == other.client_topic;
	let c3 = self.endpoint == other.endpoint;
	let c4 = self.client_interface_type == other.client_interface_type;
	c1 && c2 && c3 && c4 

    }
}

impl Ord for SubscribeRequest {
    fn cmp(&self, other: &Self) -> Ordering {
	let cmp = self.correlation_id.cmp(&other.correlation_id);
	if cmp==Ordering::Equal{
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
    NoService(String),
}

#[derive(Debug)]
pub enum ClientCallStatusCodes {
    Ok(String),
    Error(String),
}

impl fmt::Display for BackendStatusCodes {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            BackendStatusCodes::Ok(msg) => write!(f, "BackendStatusCodes::Ok {}", msg),
            BackendStatusCodes::Error(msg) => write!(f, "BackendStatusCodes::ERR {}", msg),
            BackendStatusCodes::NoTopic(msg) => write!(f, "BackendStatusCodes::NoTopic {}", msg),
	    BackendStatusCodes::NoService(msg) => write!(f, "BackendStatusCodes::NoService {}", msg),
	    
        }
    }
}

#[derive(Debug)]
pub struct MessagingResult {
    pub(crate) correlation_id: String,
    pub(crate) status: BackendStatusCodes,
}




pub struct PersistenceResult {
    pub(crate) correlation_id: String,
    pub(crate) status: BackendStatusCodes,
    pub(crate) payload: Vec<u8>
}

pub struct SIResult {
    pub(crate) correlation_id: String,
    pub(crate) status: BackendStatusCodes,
    pub(crate) payload: Vec<u8>
}

impl fmt::Display for PersistenceResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
	write!(f, "PersistenceResult {{ correlation_id: {}, status :  {}}}", &self.correlation_id, &self.status)
    }
}

impl fmt::Debug for PersistenceResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
	write!(f, "PersistenceResult {{ correlation_id: {}, status :  {}}}", &self.correlation_id, &self.status)
    }
}

impl fmt::Debug for SIResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
	write!(f, "SIResult {{ correlation_id: {}, status :  {}}}", &self.correlation_id, &self.status)
    }
}


#[derive(Debug)]
pub enum Job {
    Subscribe(SubscribeRequest),
    Unsubscribe(SubscribeRequest),
    Publish(PublishRequest),

}

#[derive(Debug)]
pub enum PersistenceJobType {
    Store(StoreRequest),
    Retrieve(RetrieveRequest),
    Delete(DeleteRequest)

}



#[derive(Debug)]
pub struct ServiceInvokeRequest {
    pub method: HttpMethod,
    pub request_target: String,
    pub headers: std::collections::HashMap<String,String>,
    pub payload: Vec<u8>	
}

pub struct ServiceInvokeResponse {
    pub method: HttpMethod,
    pub request_target: String,
    pub headers: std::collections::HashMap<String,String>,
    pub payload: Vec<u8>	
}

#[derive(Debug,Deserialize)]
pub enum HttpMethod{
    POST,
    GET,
    DELETE,
    PUT
}

impl Default for HttpMethod{
    fn default()->Self{
	HttpMethod::POST
    }
}

use std::str::FromStr;

custom_error!{pub HTTPMethodConversionError
    InvalidMethod = "Invalid HTTP method"
}

impl FromStr for HttpMethod{
    type Err = HTTPMethodConversionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
	match s{
	    "POST" => Ok(HttpMethod::POST),
	    "GET" => Ok(HttpMethod::GET),
	    "DELETE" => Ok(HttpMethod::DELETE),
	    "PUT" => Ok(HttpMethod::PUT),
	    _ => Err(HTTPMethodConversionError::InvalidMethod)
	}
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug,Default)]
pub struct RESTRequestParams{
    pub payload: Vec<u8>,
    pub uri: String,
    pub headers: std::collections::HashMap<String,String>,
    pub method: HttpMethod	
}

#[derive(Debug,Default)]
pub struct RESTResponseParams{
    pub payload: Vec<u8>,
    pub headers: std::collections::HashMap<String,String>,
    pub status: u16
}


#[derive(Debug)]
pub struct RESTRequestResult {
    pub(crate) correlation_id: String,
    pub(crate) status: ClientCallStatusCodes,
    pub(crate) response_params: RESTResponseParams 
    
}


#[derive(Debug)]
pub enum SIJobType {
    PublicInvokeHttp{
	correlation_id: String,
	service_name: String,	
	req: ServiceInvokeRequest

    },
    PublicInvokeGrpc{
	correlation_id: String,
	service_name: String,	
	req: ServiceInvokeRequest
    },
    InternalInvoke{
	correlation_id: String,
	service_name: String,	
	req: ServiceInvokeRequest
    }        
}


#[derive(Debug)]
pub struct CustomContext;

#[derive(Debug)]
pub struct RestToMessagingContext {
    pub job: Job,
    pub sender: Sender<MessagingResult>,
}

#[derive(Debug)]
pub struct BackendToRestContext {
    pub correlation_id : String,
    pub sender: Option<Sender<RESTRequestResult>>,
    pub request_params: RESTRequestParams,        
}

#[derive(Debug)]
pub struct RestToPersistenceContext {
    pub job: PersistenceJobType,
    pub sender: Sender<PersistenceResult>,
}

#[derive(Debug)]
pub struct RestToSIContext {
    pub job: SIJobType,
    pub sender: Sender<SIResult>,
}
