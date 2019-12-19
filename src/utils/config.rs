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
    pub client_topic: String,
}

#[derive(Debug, Deserialize)]
pub struct ConsumerTopic {
    pub consumer_topic: String,
    pub consumer_group: String,
    pub client_topic: String,
}

#[derive(Debug, Deserialize)]
pub struct Kafka {
    pub brokers: Vec<String>,
    pub producer_topics: Vec<ProducerTopic>,
    pub consumer_topics: Vec<ConsumerTopic>,
}

impl Kafka {
    pub fn get_producer_topic_for_client_topic(&self, client_topic: &String) -> Option<String> {
        let mut i: usize = 0;
        let mut maybe_topic = None;
        for t in self.producer_topics.iter() {
            if t.client_topic.eq(client_topic) {
                maybe_topic = Some(t.producer_topic.clone());
            }
            i = i + 1;
        }
        maybe_topic
    }

    pub fn get_consumer_topic_for_client_topic(&self, client_topic: &String) -> Option<String> {
        let mut i: usize = 0;
        let mut maybe_topic = None;
        for t in self.consumer_topics.iter() {
            if t.client_topic.eq(client_topic) {
                maybe_topic = Some(t.consumer_topic.clone());
            }
            i = i + 1;
        }
        maybe_topic
    }
}

#[derive(Debug, Deserialize)]
pub struct Nats {
    pub brokers: Vec<String>,
    pub producer_topics: Vec<ProducerTopic>,
    pub consumer_topics: Vec<ConsumerTopic>,
}

impl Nats {
    pub fn get_producer_topic_for_client_topic(&self, client_topic: &String) -> Option<String> {
        let mut i: usize = 0;
        let mut maybe_topic = None;
        for t in self.producer_topics.iter() {
            if t.client_topic.eq(client_topic) {
                maybe_topic = Some(t.producer_topic.clone());
            }
            i = i + 1;
        }
        maybe_topic
    }

    pub fn get_consumer_topic_for_client_topic(&self, client_topic: &String) -> Option<String> {
        let mut i: usize = 0;
        let mut maybe_topic = None;
        for t in self.consumer_topics.iter() {
            if t.client_topic.eq(client_topic) {
                maybe_topic = Some(t.consumer_topic.clone());
            }
            i = i + 1;
        }
        maybe_topic
    }
}

#[derive(Debug, Deserialize)]
pub struct Channels {
    pub kafka: Vec<Kafka>,
    pub nats: Vec<Nats>,
}

#[derive(Debug)]
pub struct MemoryChannelEndpoint {
    pub from_client_receiver: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>,
    pub to_client_sender_for_rest: Box<mpsc::Sender<MessagingToRestContext>>,
    pub to_client_sender_for_grpc: Box<mpsc::Sender<MessagingToRestContext>>,
}

#[derive(Debug)]
pub struct MemoryChannel {
    pub kafka_memory_channels: Vec<MemoryChannelEndpoint>,
    pub nats_memory_channels: Vec<MemoryChannelEndpoint>,
    pub to_client_receiver_for_rest: Arc<Mutex<mpsc::Receiver<MessagingToRestContext>>>,
    pub to_client_receiver_for_grpc: Arc<Mutex<mpsc::Receiver<MessagingToRestContext>>>,
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
    let (to_client_sender_for_rest, to_client_receiver_for_rest): (
        mpsc::Sender<MessagingToRestContext>,
        mpsc::Receiver<MessagingToRestContext>,
    ) = mpsc::channel(1000);

    let (to_client_sender_for_grpc, to_client_receiver_for_grpc): (
        mpsc::Sender<MessagingToRestContext>,
        mpsc::Receiver<MessagingToRestContext>,
    ) = mpsc::channel(1000);

    let to_client_sender_for_rest = Box::new(to_client_sender_for_rest);
    let to_client_receiver_for_rest = Arc::new(Mutex::new(to_client_receiver_for_rest));

    let to_client_sender_for_grpc = Box::new(to_client_sender_for_grpc);
    let to_client_receiver_for_grpc = Arc::new(Mutex::new(to_client_receiver_for_grpc));

    let mut kafka_memory_channels = vec![];
    let mut from_client_to_backend_channel_sender = HashMap::new();

    for kafka_channels in config.channels.kafka.iter() {
        let (from_client_sender, from_client_receiver): (
            mpsc::Sender<RestToMessagingContext>,
            mpsc::Receiver<RestToMessagingContext>,
        ) = mpsc::channel(1000);
        let mme = MemoryChannelEndpoint {
            from_client_receiver: Arc::new(Mutex::new(from_client_receiver)),
            to_client_sender_for_rest: to_client_sender_for_rest.clone(),
            to_client_sender_for_grpc: to_client_sender_for_grpc.clone(),
        };
        let from_client_sender = Box::new(from_client_sender);

        kafka_memory_channels.push(mme);
        for producer_topic in kafka_channels.producer_topics.iter() {
            from_client_to_backend_channel_sender.insert(
                producer_topic.client_topic.clone(),
                from_client_sender.clone(),
            );
        }

        for consumer_topic in kafka_channels.consumer_topics.iter() {
            from_client_to_backend_channel_sender.insert(
                consumer_topic.client_topic.clone(),
                from_client_sender.clone(),
            );
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
            to_client_sender_for_rest: to_client_sender_for_rest.clone(),
            to_client_sender_for_grpc: to_client_sender_for_grpc.clone(),
        };
        nats_memory_channels.push(mme);
        for producer_topic in nats_channels.producer_topics.iter() {
            from_client_to_backend_channel_sender.insert(
                producer_topic.client_topic.clone(),
                from_client_sender.clone(),
            );
        }

        for consumer_topic in nats_channels.consumer_topics.iter() {
            from_client_to_backend_channel_sender.insert(
                consumer_topic.client_topic.clone(),
                from_client_sender.clone(),
            );
        }
    }

    let mc = MemoryChannel {
        kafka_memory_channels,
        nats_memory_channels,
        to_client_receiver_for_rest,
        to_client_receiver_for_grpc,
        from_client_to_backend_channel_sender: Box::new(from_client_to_backend_channel_sender),
    };

    debug! {"MC {:?}", mc};
    mc
}
