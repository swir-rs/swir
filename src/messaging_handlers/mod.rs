use std::thread::JoinHandle;

use crossbeam_channel::Receiver;
use sled::Db;

use super::utils::structs::InternalMessage;

pub type Result<T> = std::result::Result<T, String>;

#[cfg(feature = "with_nats")]
mod nats_handler;

#[cfg(feature = "with_nats")]
pub fn configure_broker(broker_address: String, sending_topic: String, receiving_topic: String, receiving_group: String, db: Db, rx: Receiver<InternalMessage>) -> Result<Vec<JoinHandle<()>>> {
    nats_handler::configure_broker(broker_address, sending_topic, receiving_topic, receiving_group, db, rx)
}

#[cfg(not(feature = "with_nats"))]
mod kafka_handler;

#[cfg(not(feature = "with_nats"))]
pub fn configure_broker(broker_address: String, sending_topic: String, receiving_topic: String, receiving_group: String, db: Db, rx: Receiver<InternalMessage>) -> Result<Vec<JoinHandle<()>>> {
    kafka_handler::configure_broker(broker_address, sending_topic, receiving_topic, receiving_group, db, rx)
}
