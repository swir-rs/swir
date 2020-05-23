use crate::utils::config::{StoreType, Stores};
use crate::utils::structs::RestToPersistenceContext;
use async_trait::async_trait;
use futures::future::join_all;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{
    mpsc,
    Mutex
};

mod dynamodb_store;
mod redis_store;

#[async_trait]
trait Store {
    async fn configure_store(&self);
}

pub async fn configure_stores(stores: Stores, from_client_to_persistence_receivers: HashMap<StoreType, Vec<Arc<Mutex<mpsc::Receiver<RestToPersistenceContext>>>>>) {
    let mut futures = vec![];

    let receivers = from_client_to_persistence_receivers.get(&StoreType::Redis);
    if let Some(receivers) = receivers {
        for (i, redis_store) in stores.redis.iter().enumerate() {
            let receiver = &receivers[i];
            let redis_store = redis_store::RedisStore::new(redis_store.to_owned(), receiver.clone());
            futures.push(tokio::spawn(async move { redis_store.configure_store().await }));
        }
    }

    let receivers = from_client_to_persistence_receivers.get(&StoreType::DynamoDb);
    if let Some(receivers) = receivers {
        for (i, dynamodb_store) in stores.dynamodb.iter().enumerate() {
            let receiver = &receivers[i];
            let dynamo_store = dynamodb_store::DynamoDbStore::new(dynamodb_store.to_owned(), receiver.clone());
            futures.push(tokio::spawn(async move { dynamo_store.configure_store().await }));
        }
    }

    debug!("Store handlers configured {}", futures.len());

    join_all(futures).await;
}
