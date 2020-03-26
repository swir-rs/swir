use std::collections::HashMap;
use async_trait::async_trait;
use crate::utils::config::{Stores, StoreType};
use crate::utils::structs::{RestToPersistenceContext};
use tokio::sync::mpsc;
use futures::future::join_all;
use std::sync::Arc;
use futures::lock::Mutex;


mod redis_store;
mod dynamodb_store;

#[async_trait]
trait Store {
    async fn configure_store(&self);
}

pub async fn configure_stores(stores: Stores, from_client_to_persistence_receivers:HashMap<StoreType, Vec<Arc<Mutex<mpsc::Receiver<RestToPersistenceContext>>>>>) {
    let mut store_handlers: Vec<Box<dyn Store>> = vec![];
    let mut futures = vec![];

    let receivers = from_client_to_persistence_receivers.get(&StoreType::Redis);
    if let Some(receivers) = receivers{
	for (i,redis_store) in stores.redis.iter().enumerate(){
	    let receiver = &receivers[i];
	    let redis_store = redis_store::RedisStore::new(redis_store,receiver.clone());
	    store_handlers.push(Box::new(redis_store));	  
	}
    }

    let receivers = from_client_to_persistence_receivers.get(&StoreType::DynamoDb);
    if let Some(receivers) = receivers{
	for (i,dynamodb_store) in stores.dynamodb.iter().enumerate(){
	    let receiver = &receivers[i];
	    let redis_store = dynamodb_store::DynamoDbStore::new(dynamodb_store,receiver.clone());
	    store_handlers.push(Box::new(redis_store));
	}
    }

    debug!("Store handlers to configure {}", store_handlers.len());
    for store_handler in store_handlers.iter() {
        let f = async move { store_handler.configure_store().await };
        futures.push(f);
    }
    join_all(futures).await;
}

