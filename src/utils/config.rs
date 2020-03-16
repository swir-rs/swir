use config::{Config, File};
use futures::lock::Mutex;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::utils::structs::{MessagingToRestContext, RestToMessagingContext};

#[derive(Debug, Deserialize,Clone)]
pub struct ProducerTopic {
    pub producer_topic: String,
    pub client_topic: String,
}

#[derive(Debug, Deserialize,Clone)]
pub struct ConsumerTopic {
    pub consumer_topic: String,
    pub consumer_group: String,
    pub client_topic: String,
}

#[derive(Debug, Deserialize,Clone)]
pub struct Kafka {
    pub brokers: Vec<String>,
    pub producer_topics: Vec<ProducerTopic>,
    pub consumer_topics: Vec<ConsumerTopic>,
}

pub trait ClientTopicsConfiguration {
    fn get_producer_topic_for_client_topic(&self, client_topic: &String) -> Option<String>;
    fn get_consumer_topic_for_client_topic(&self, client_topic: &String) -> Option<String>;
}

impl ClientTopicsConfiguration for Kafka {
    fn get_producer_topic_for_client_topic(&self, client_topic: &String) -> Option<String> {
        let mut maybe_topic = None;
        for t in self.producer_topics.iter() {
            if t.client_topic.eq(client_topic) {
                maybe_topic = Some(t.producer_topic.clone());
            }
        }
        maybe_topic
    }

    fn get_consumer_topic_for_client_topic(&self, client_topic: &String) -> Option<String> {
        let mut maybe_topic = None;
        for t in self.consumer_topics.iter() {
            if t.client_topic.eq(client_topic) {
                maybe_topic = Some(t.consumer_topic.clone());
            }
        }
        maybe_topic
    }
}

#[derive(Debug, Deserialize,Clone)]
pub struct Nats {
    pub brokers: Vec<String>,
    pub producer_topics: Vec<ProducerTopic>,
    pub consumer_topics: Vec<ConsumerTopic>,
}

impl ClientTopicsConfiguration for Nats {
    fn get_producer_topic_for_client_topic(&self, client_topic: &String) -> Option<String> {
        let mut maybe_topic = None;
        for t in self.producer_topics.iter() {
            if t.client_topic.eq(client_topic) {
                maybe_topic = Some(t.producer_topic.clone());
            }
        }
        maybe_topic
    }

    fn get_consumer_topic_for_client_topic(&self, client_topic: &String) -> Option<String> {
        let mut maybe_topic = None;
        for t in self.consumer_topics.iter() {
            if t.client_topic.eq(client_topic) {
                maybe_topic = Some(t.consumer_topic.clone());
            }
        }
        maybe_topic
    }
}

#[derive(Debug, Deserialize,Clone)]
pub struct AwsKinesis {

    pub regions: Vec<String>,
    pub producer_topics: Vec<ProducerTopic>,
    pub consumer_topics: Vec<ConsumerTopic>,
}

impl ClientTopicsConfiguration for AwsKinesis {
    fn get_producer_topic_for_client_topic(&self, client_topic: &String) -> Option<String> {
        let mut maybe_topic = None;
        for t in self.producer_topics.iter() {
            if t.client_topic.eq(client_topic) {
                maybe_topic = Some(t.producer_topic.clone());
            }
        }
        maybe_topic
    }

    fn get_consumer_topic_for_client_topic(&self, client_topic: &String) -> Option<String> {
        let mut maybe_topic = None;
        for t in self.consumer_topics.iter() {
            if t.client_topic.eq(client_topic) {
                maybe_topic = Some(t.consumer_topic.clone());
            }
        }
        maybe_topic
    }
}

#[derive(Debug, Deserialize)]
pub struct Channels {
    pub kafka: Vec<Kafka>,
    pub nats: Vec<Nats>,
    pub aws_kinesis: Vec<AwsKinesis>,
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
    pub aws_kinesis_memory_channels: Vec<MemoryChannelEndpoint>,
    pub to_client_sender_for_rest: Box<HashMap<String,Box<mpsc::Sender<MessagingToRestContext>>>>,
    pub to_client_receiver_for_rest: Arc<Mutex<mpsc::Receiver<MessagingToRestContext>>>,
    pub to_client_receiver_for_grpc: Arc<Mutex<mpsc::Receiver<MessagingToRestContext>>>,
    pub from_client_to_backend_channel_sender: Box<HashMap<String, Box<mpsc::Sender<RestToMessagingContext>>>>,
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
	s.merge(config::Environment::with_prefix("SWIR")).unwrap();
	debug!("{:?}",s);
	
	match s.get_str("config_file"){
	    Ok(config_file)=>{
		debug!("Trying config file at : {}", config_file);
		s.merge(File::with_name(&config_file)).unwrap();
	    },
	    Err(e)=>{
		debug!("Trying default config file ./swir.yaml {:?}",e);
		s.merge(File::with_name("swir.yaml")).unwrap();
	    }
	}
	    	
        let s = s.try_into().unwrap();
        info!("SWIR Config : {:?}", s);
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
//  For REST frontend there is only one channel and all subscriptions are using it since the receiving side of that channel is with Hyper Rest Client
//  
//                     <----------------- broker 1 < ---- to_client_sender
//  to_client_receiver <----------------- broker 2 < ---- to_client_sender
//                     <----------------- broker 3 < ---- to_client_sender

//  For gRPC there can be multiple channels since each subscribe call is creates a stream
//  
//  to_client_receiver1 <----------------- broker 1 < ---- to_client_sender1
//  to_client_receiver2 <----------------- broker 2 < ---- to_client_sender2
//  to_client_receiver3 <----------------- broker 3 < ---- to_client_sender3


pub fn create_client_to_backend_channels(config: &Box<Swir>) -> MemoryChannel {
    let (to_client_sender_for_rest, to_client_receiver_for_rest): (mpsc::Sender<MessagingToRestContext>, mpsc::Receiver<MessagingToRestContext>) = mpsc::channel(20000);


    let mut to_client_sender_for_rest_map = HashMap::new();

    let (to_client_sender_for_grpc, to_client_receiver_for_grpc): (mpsc::Sender<MessagingToRestContext>, mpsc::Receiver<MessagingToRestContext>) = mpsc::channel(20000);

    let box_to_client_sender_for_rest = Box::new(to_client_sender_for_rest);
    let to_client_receiver_for_rest = Arc::new(Mutex::new(to_client_receiver_for_rest));

    let to_client_sender_for_grpc = Box::new(to_client_sender_for_grpc);
    let to_client_receiver_for_grpc = Arc::new(Mutex::new(to_client_receiver_for_grpc));

    let mut kafka_memory_channels = vec![];
    let mut from_client_to_backend_channel_sender = HashMap::new();

    for kafka_channels in config.channels.kafka.iter() {
        let (from_client_sender, from_client_receiver): (mpsc::Sender<RestToMessagingContext>, mpsc::Receiver<RestToMessagingContext>) = mpsc::channel(20000);
        let mme = MemoryChannelEndpoint {
            from_client_receiver: Arc::new(Mutex::new(from_client_receiver)),
            to_client_sender_for_rest: box_to_client_sender_for_rest.clone(),
            to_client_sender_for_grpc: to_client_sender_for_grpc.clone(),
        };
        let from_client_sender = Box::new(from_client_sender);

        kafka_memory_channels.push(mme);
        for producer_topic in kafka_channels.producer_topics.iter() {
            from_client_to_backend_channel_sender.insert(producer_topic.client_topic.clone(), from_client_sender.clone());
        }

        for consumer_topic in kafka_channels.consumer_topics.iter() {
            from_client_to_backend_channel_sender.insert(consumer_topic.client_topic.clone(), from_client_sender.clone());
	    to_client_sender_for_rest_map.insert(consumer_topic.client_topic.clone(),box_to_client_sender_for_rest.clone());
        }
    }


    let mut aws_kinesis_memory_channels = vec![];
    for aws_kinesis_channels in config.channels.aws_kinesis.iter() {
        let (from_client_sender, from_client_receiver): (mpsc::Sender<RestToMessagingContext>, mpsc::Receiver<RestToMessagingContext>) = mpsc::channel(20000);
        let mme = MemoryChannelEndpoint {
            from_client_receiver: Arc::new(Mutex::new(from_client_receiver)),
            to_client_sender_for_rest: box_to_client_sender_for_rest.clone(),
            to_client_sender_for_grpc: to_client_sender_for_grpc.clone(),
        };
        let from_client_sender = Box::new(from_client_sender);

        aws_kinesis_memory_channels.push(mme);
        for producer_topic in aws_kinesis_channels.producer_topics.iter() {
            from_client_to_backend_channel_sender.insert(producer_topic.client_topic.clone(), from_client_sender.clone());
        }

        for consumer_topic in aws_kinesis_channels.consumer_topics.iter() {
            from_client_to_backend_channel_sender.insert(consumer_topic.client_topic.clone(), from_client_sender.clone());
	    to_client_sender_for_rest_map.insert(consumer_topic.client_topic.clone(),box_to_client_sender_for_rest.clone());
        }
    }
    
    
    let mut nats_memory_channels = vec![];
    #[cfg(feature = "with_nats")]
    {
        for nats_channels in config.channels.nats.iter() {
            let (from_client_sender, from_client_receiver): (mpsc::Sender<RestToMessagingContext>, mpsc::Receiver<RestToMessagingContext>) = mpsc::channel(20000);

            let from_client_sender = Box::new(from_client_sender);

            let mme = MemoryChannelEndpoint {
                from_client_receiver: Arc::new(Mutex::new(from_client_receiver)),
                to_client_sender_for_rest: box_to_client_sender_for_rest.clone(),
                to_client_sender_for_grpc: to_client_sender_for_grpc.clone(),
            };
            nats_memory_channels.push(mme);
            for producer_topic in nats_channels.producer_topics.iter() {
                from_client_to_backend_channel_sender.insert(producer_topic.client_topic.clone(), from_client_sender.clone());
            }

            for consumer_topic in nats_channels.consumer_topics.iter() {
                from_client_to_backend_channel_sender.insert(consumer_topic.client_topic.clone(), from_client_sender.clone());
		to_client_sender_for_rest_map.insert(consumer_topic.client_topic.clone(),box_to_client_sender_for_rest.clone());
            }
        }
    }

    let mc = MemoryChannel {
        kafka_memory_channels,
        nats_memory_channels,
	aws_kinesis_memory_channels,
	to_client_sender_for_rest:Box::new(to_client_sender_for_rest_map),
        to_client_receiver_for_rest,
        to_client_receiver_for_grpc,
        from_client_to_backend_channel_sender: Box::new(from_client_to_backend_channel_sender),
    };

    debug! {"MC {:?}", mc};
    mc
}
