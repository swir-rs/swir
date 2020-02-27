use crate::utils::structs::RestToMessagingContext;
use crate::utils::structs::CustomerInterfaceType;
use crate::utils::structs::MessagingToRestContext;
use std::collections::HashMap;
use futures::lock::Mutex;
use std::sync::Arc;
use sled::Db;
use async_trait::async_trait;
use crate::messaging_handlers::Broker;
use tokio::sync::mpsc;

pub struct GrpcBroker {
    pub db: Db,
    pub rx: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>,
    pub tx: Box<HashMap<CustomerInterfaceType, Box<mpsc::Sender<MessagingToRestContext>>>>,
}


#[async_trait]
impl Broker for GrpcBroker{

}
