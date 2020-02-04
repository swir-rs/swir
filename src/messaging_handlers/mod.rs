use std::collections::HashMap;

use futures::future::join_all;
use sled::Db;
use std::sync::Arc;
use async_trait::async_trait;
use futures::lock::Mutex;
use crate::utils::config::{Channels, MemoryChannel};
use crate::utils::structs::CustomerInterfaceType;

mod kafka_handler;
#[cfg(feature = "with_nats")]
mod nats_handler;

#[async_trait]
trait Broker {
    async fn configure_broker(&self);
}

pub async fn configure_broker(messaging: Channels, db: Db, mc: MemoryChannel) {
    let mut brokers: Vec<Box<dyn Broker>> = vec![];
    let mut futures = vec![];

    let mut i = 0;
    for kafka in messaging.kafka {
        let mce = mc.kafka_memory_channels.get(i);
        i = i + 1;
        if let None = mce {
            continue;
        }
        let mce = mce.unwrap();
        let rx = mce.from_client_receiver.to_owned();
        let tx_rest = mce.to_client_sender_for_rest.to_owned();
        let tx_grpc = mce.to_client_sender_for_grpc.to_owned();
        let mut tx_map = HashMap::new();
        tx_map.insert(CustomerInterfaceType::REST, tx_rest);
        tx_map.insert(CustomerInterfaceType::GRPC, tx_grpc);

        let kafka_broker = kafka_handler::KafkaBroker {
            kafka,
            db: db.clone(),
            rx,
            tx: Box::new(tx_map),
	    subscriptions: Arc::new(Mutex::new(Box::new(HashMap::new()))),
        };
        brokers.push(Box::new(kafka_broker));
    }

    #[cfg(feature = "with_nats")]
    {
        i = 0;
        for nats in messaging.nats {
            let mce = mc.nats_memory_channels.get(i);
            i = i + 1;
            if let None = mce {
                warn!("No memory channel for {:?}", nats);
                continue;
            }
            let mce = mce.unwrap();
            let rx = mce.from_client_receiver.to_owned();
            let tx_rest = mce.to_client_sender_for_rest.to_owned();
            let tx_grpc = mce.to_client_sender_for_grpc.to_owned();
            let mut tx_map = HashMap::new();
            tx_map.insert(CustomerInterfaceType::REST, tx_rest);
            tx_map.insert(CustomerInterfaceType::GRPC, tx_grpc);

            let nats_broker = nats_handler::NatsBroker {
                nats,
                db: db.clone(),
                rx,
                tx: Box::new(tx_map),
            };
            brokers.push(Box::new(nats_broker));
        }
    }

    debug!("Brokers to configure {}", brokers.len());
    for broker in brokers.iter() {
        let f = async move { broker.configure_broker().await };
        futures.push(f);
    }

    join_all(futures).await;
}
