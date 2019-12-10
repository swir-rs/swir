use futures::channel::mpsc::{Receiver, Sender};
use sled::Db;

use crate::utils;

use super::utils::structs::RestToMessagingContext;

#[cfg(feature = "with_nats")]
mod nats_handler;

#[cfg(feature = "with_nats")]
pub async fn configure_broker(broker_address: String, sending_topic: String, receiving_topic: String, receiving_group: String, db: Db, rx: Receiver<RestToMessagingContext>, tx: Sender<utils::structs::MessagingToRestContext>) {
    nats_handler::configure_broker(broker_address, sending_topic, receiving_topic, receiving_group, db, rx, tx).await
}

#[cfg(not(feature = "with_nats"))]
mod kafka_handler;

#[cfg(not(feature = "with_nats"))]
pub async fn configure_broker(broker_address: String, sending_topic: String, receiving_topic: String, receiving_group: String, db: Db, rx: Receiver<RestToMessagingContext>, tx: Sender<utils::structs::MessagingToRestContext>) {
    kafka_handler::configure_broker(broker_address, sending_topic, receiving_topic, receiving_group, db, rx, tx).await
}
