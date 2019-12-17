use std::collections::HashMap;
use std::sync::Arc;

use config::{Config, File};
use futures::lock::Mutex;
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::utils::structs::{MessagingToRestContext, RestToMessagingContext};

#[derive(Debug, Deserialize)]
pub struct ProducerTopic {
    pub producer_topic: String,
    pub client_topics: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConsumerTopic {
    pub consumer_topic: String,
    pub consumer_group: String,
    pub client_topics: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Kafka {
    pub brokers: Vec<String>,
    pub producer_topics: Vec<ProducerTopic>,
    pub consumer_topics: Vec<ConsumerTopic>,
}

#[derive(Debug, Deserialize)]
pub struct Nats {
    pub brokers: Vec<String>,
    pub producer_topics: Vec<ProducerTopic>,
    pub consumer_topics: Vec<ConsumerTopic>,
}

#[derive(Debug, Deserialize)]
pub struct Channels {
    pub kafka: Vec<Kafka>,
    pub nats: Vec<Nats>,
}

#[derive(Debug)]
pub struct MemoryChannelEndpoint {
    pub from_client_receiver: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>,
    pub to_client_sender: Box<mpsc::Sender<MessagingToRestContext>>,
}

#[derive(Debug)]
pub struct MemoryChannel {
    pub kafka_memory_channels: Vec<MemoryChannelEndpoint>,
    pub nats_memory_channels: Vec<MemoryChannelEndpoint>,
    pub to_client_receiver: Arc<Mutex<mpsc::Receiver<MessagingToRestContext>>>,
    pub from_client_to_backend_channel_sender:
        Box<HashMap<String, Box<mpsc::Sender<RestToMessagingContext>>>>,
}

#[derive(Debug, Deserialize)]
pub struct Swir {
    pub client_ip: String,
    pub client_http_port: u16,
    pub client_https_port: u16,
    pub client_grpc_port: u16,
    pub client_tls_private_key: String,
    pub client_tls_certificate: String,
    pub client_executable: Option<String>,
    pub channels: Channels,
}

impl Swir {
    pub fn new() -> Box<Swir> {
        let mut s = Config::new();
        s.merge(File::with_name("swir.yaml")).unwrap();
        let s = s.try_into().unwrap();
        println!("SWIR Config : {:?}", s);
        Box::new(s)
    }
}

//
//  due to the nature of MPSC we have following
//
//  from_client_sender [client topic1] ------->  broker 1
//  from_client_sender [client topic2] ------->  broker 1 --> from_client_receiver
//  from_client_sender [client topic3] ------->  broker 1

//  from_client_sender [client topic1] ------->  broker 2
//  from_client_sender [client topic2] ------->  broker 2 --> from_client_receiver
//  from_client_sender [client topic3] ------->  broker 2
//
//
//                     <----------------- broker 1 < ---- to_client_sender
//  to_client_receiver <----------------- broker 2 < ---- to_client_sender
//                     <----------------- broker 3 < ---- to_client_sender
//
//
//

pub fn create_client_to_backend_channels(config: &Box<Swir>) -> MemoryChannel {
    let (to_client_sender, to_client_receiver): (
        mpsc::Sender<MessagingToRestContext>,
        mpsc::Receiver<MessagingToRestContext>,
    ) = mpsc::channel(1000);

    let to_client_sender = Box::new(to_client_sender);
    let to_client_receiver = Arc::new(Mutex::new(to_client_receiver));

    let mut kafka_memory_channels = vec![];
    let mut from_client_to_backend_channel_sender = HashMap::new();

    for kafka_channels in config.channels.kafka.iter() {
        let (from_client_sender, from_client_receiver): (
            mpsc::Sender<RestToMessagingContext>,
            mpsc::Receiver<RestToMessagingContext>,
        ) = mpsc::channel(1000);
        let mme = MemoryChannelEndpoint {
            from_client_receiver: Arc::new(Mutex::new(from_client_receiver)),
            to_client_sender: to_client_sender.clone(),
        };
        let from_client_sender = Box::new(from_client_sender);

        kafka_memory_channels.push(mme);
        for producer_topic in kafka_channels.producer_topics.iter() {
            for client_topic in producer_topic.client_topics.iter() {
                from_client_to_backend_channel_sender
                    .insert(client_topic.clone(), from_client_sender.clone());
            }
        }
    }
    let mut nats_memory_channels = vec![];
    for nats_channels in config.channels.nats.iter() {
        let (from_client_sender, from_client_receiver): (
            mpsc::Sender<RestToMessagingContext>,
            mpsc::Receiver<RestToMessagingContext>,
        ) = mpsc::channel(1000);

        let from_client_sender = Box::new(from_client_sender);

        let mme = MemoryChannelEndpoint {
            from_client_receiver: Arc::new(Mutex::new(from_client_receiver)),
            to_client_sender: to_client_sender.clone(),
        };
        nats_memory_channels.push(mme);
        for producer_topic in nats_channels.producer_topics.iter() {
            for client_topic in producer_topic.client_topics.iter() {
                from_client_to_backend_channel_sender
                    .insert(client_topic.clone(), from_client_sender.clone());
            }
        }
    }

    let mc = MemoryChannel {
        kafka_memory_channels,
        nats_memory_channels,
        to_client_receiver,
        from_client_to_backend_channel_sender: Box::new(from_client_to_backend_channel_sender),
    };

    debug! {"MC {:?}", mc};
    mc
}
