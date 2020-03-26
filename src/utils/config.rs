use config::{Config, File};
use futures::lock::Mutex;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::utils::structs::{MessagingToRestContext, RestToMessagingContext,RestToPersistenceContext};

#[derive(Hash,PartialEq,Eq)]
pub enum StoreType {
    Redis,
    DynamoDb,
}

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
    fn get_producer_topic_for_client_topic(&self, client_topic: &str) -> Option<String>;
    fn get_consumer_topic_for_client_topic(&self, client_topic: &str) -> Option<String>;
}

impl ClientTopicsConfiguration for Kafka {
    fn get_producer_topic_for_client_topic(&self, client_topic: &str) -> Option<String> {
        let mut maybe_topic = None;
        for t in self.producer_topics.iter() {
            if t.client_topic.eq(client_topic) {
                maybe_topic = Some(t.producer_topic.clone());
            }
        }
        maybe_topic
    }

    fn get_consumer_topic_for_client_topic(&self, client_topic: &str) -> Option<String> {
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
    fn get_producer_topic_for_client_topic(&self, client_topic: &str) -> Option<String> {
        let mut maybe_topic = None;
        for t in self.producer_topics.iter() {
            if t.client_topic.eq(client_topic) {
                maybe_topic = Some(t.producer_topic.clone());
            }
        }
        maybe_topic
    }

    fn get_consumer_topic_for_client_topic(&self, client_topic: &str) -> Option<String> {
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
    fn get_producer_topic_for_client_topic(&self, client_topic: &str) -> Option<String> {
        let mut maybe_topic = None;
        for t in self.producer_topics.iter() {
            if t.client_topic.eq(client_topic) {
                maybe_topic = Some(t.producer_topic.clone());
            }
        }
        maybe_topic
    }

    fn get_consumer_topic_for_client_topic(&self, client_topic: &str) -> Option<String> {
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
pub struct Redis {
    pub nodes: Vec<String>,
    pub client_database_names: Vec<String>,
}

#[derive(Debug, Deserialize,Clone)]
pub struct DynamoDb {
    pub client_database_names: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Channels {
    pub kafka: Vec<Kafka>,
    pub nats: Vec<Nats>,
    pub aws_kinesis: Vec<AwsKinesis>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Stores {
    pub redis: Vec<Redis>,
    pub dynamodb: Vec<DynamoDb>,
}

#[derive(Debug)]
pub struct MemoryChannelEndpoint<T1,T2> {
    pub from_client_receiver: Arc<Mutex<mpsc::Receiver<T1>>>,
    pub to_client_sender_for_rest: mpsc::Sender<T2>,
    pub to_client_sender_for_grpc: mpsc::Sender<T2>
}

pub struct MessagingMemoryChannels {
    pub kafka_memory_channels: Vec<MemoryChannelEndpoint<RestToMessagingContext,MessagingToRestContext>>,
    pub nats_memory_channels: Vec<MemoryChannelEndpoint<RestToMessagingContext,MessagingToRestContext>>,
    pub aws_kinesis_memory_channels: Vec<MemoryChannelEndpoint<RestToMessagingContext,MessagingToRestContext>>,
    pub to_client_sender_for_rest: HashMap<String,mpsc::Sender<MessagingToRestContext>>,
    pub to_client_receiver_for_rest: Arc<Mutex<mpsc::Receiver<MessagingToRestContext>>>,
    pub to_client_receiver_for_grpc: Arc<Mutex<mpsc::Receiver<MessagingToRestContext>>>,
    pub from_client_to_messaging_sender: HashMap<String, mpsc::Sender<RestToMessagingContext>>
}


pub struct PersistenceMemoryChannels {
    pub from_client_to_persistence_senders: HashMap<String, mpsc::Sender<RestToPersistenceContext>>,
    pub from_client_to_persistence_receivers_map: HashMap<StoreType, Vec<Arc<Mutex<mpsc::Receiver<RestToPersistenceContext>>>>>
}

pub struct MemoryChannels {
    pub messaging_memory_channels: MessagingMemoryChannels,
    pub persistence_memory_channels: PersistenceMemoryChannels,

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
    pub stores: Stores,
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


// Messaging
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



// Persistence
// from_client_sender [database_name_1] --> key-value store 1
// from_client_sender [database_name_2] --> key-value store 2


fn create_messaging_channels(config: &Swir)->MessagingMemoryChannels{
    let mut to_client_sender_for_rest_map = HashMap::new();
    let (to_client_sender_for_grpc, to_client_receiver_for_grpc): (mpsc::Sender<MessagingToRestContext>, mpsc::Receiver<MessagingToRestContext>) = mpsc::channel(20000);
    let (to_client_sender_for_rest, to_client_receiver_for_rest): (mpsc::Sender<MessagingToRestContext>, mpsc::Receiver<MessagingToRestContext>) = mpsc::channel(20000);

    let to_client_receiver_for_rest = Arc::new(Mutex::new(to_client_receiver_for_rest));    
    let to_client_sender_for_grpc = to_client_sender_for_grpc;
    let to_client_receiver_for_grpc = Arc::new(Mutex::new(to_client_receiver_for_grpc));
    let box_to_client_sender_for_rest = to_client_sender_for_rest;
    
    let mut kafka_memory_channels = vec![];
    let mut from_client_to_messaging_sender = HashMap::new();    

    for kafka_channels in config.channels.kafka.iter() {
        let (from_client_sender, from_client_receiver): (mpsc::Sender<RestToMessagingContext>, mpsc::Receiver<RestToMessagingContext>) = mpsc::channel(20000);
        let mme = MemoryChannelEndpoint {
            from_client_receiver: Arc::new(Mutex::new(from_client_receiver)),
            to_client_sender_for_rest: box_to_client_sender_for_rest.clone(),
            to_client_sender_for_grpc: to_client_sender_for_grpc.clone(),
        };

        kafka_memory_channels.push(mme);
        for producer_topic in kafka_channels.producer_topics.iter() {
            from_client_to_messaging_sender.insert(producer_topic.client_topic.clone(), from_client_sender.clone());
        }

        for consumer_topic in kafka_channels.consumer_topics.iter() {
            from_client_to_messaging_sender.insert(consumer_topic.client_topic.clone(), from_client_sender.clone());
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

        aws_kinesis_memory_channels.push(mme);
        for producer_topic in aws_kinesis_channels.producer_topics.iter() {
            from_client_to_messaging_sender.insert(producer_topic.client_topic.clone(), from_client_sender.clone());
        }

        for consumer_topic in aws_kinesis_channels.consumer_topics.iter() {
            from_client_to_messaging_sender.insert(consumer_topic.client_topic.clone(), from_client_sender.clone());
	    to_client_sender_for_rest_map.insert(consumer_topic.client_topic.clone(),box_to_client_sender_for_rest.clone());
        }
    }
    
    
    let mut nats_memory_channels = vec![];
    #[cfg(feature = "with_nats")]
    {
        for nats_channels in config.channels.nats.iter() {
            let (from_client_sender, from_client_receiver): (mpsc::Sender<RestToMessagingContext>, mpsc::Receiver<RestToMessagingContext>) = mpsc::channel(20000);


            let mme = MemoryChannelEndpoint {
                from_client_receiver: Arc::new(Mutex::new(from_client_receiver)),
                to_client_sender_for_rest: box_to_client_sender_for_rest.clone(),
                to_client_sender_for_grpc: to_client_sender_for_grpc.clone(),
            };
            nats_memory_channels.push(mme);
            for producer_topic in nats_channels.producer_topics.iter() {
                from_client_to_messaging_sender.insert(producer_topic.client_topic.clone(), from_client_sender.clone());
            }

            for consumer_topic in nats_channels.consumer_topics.iter() {
                from_client_to_messaging_sender.insert(consumer_topic.client_topic.clone(), from_client_sender.clone());
		to_client_sender_for_rest_map.insert(consumer_topic.client_topic.clone(),box_to_client_sender_for_rest.clone());
            }
        }
    }
    let mc = MessagingMemoryChannels {
        kafka_memory_channels,
        nats_memory_channels,
	aws_kinesis_memory_channels,
	to_client_sender_for_rest:to_client_sender_for_rest_map,
        to_client_receiver_for_rest,
        to_client_receiver_for_grpc,
        from_client_to_messaging_sender
    };

    mc
}


fn create_persistence_channels(config: &Swir)->PersistenceMemoryChannels{
    let mut from_client_to_persistence_senders = HashMap::new();
    let mut from_client_to_persistence_receivers_map = HashMap::new();
    let mut from_client_to_persistence_receivers = Vec::new();
    
    for redis_store in config.stores.redis.iter() {
	if !redis_store.client_database_names.is_empty(){
	    let (from_client_sender, from_client_receiver): (mpsc::Sender<RestToPersistenceContext>, mpsc::Receiver<RestToPersistenceContext>) = mpsc::channel(10);
	    for database_name in redis_store.client_database_names.iter(){
		from_client_to_persistence_senders.insert(database_name.clone(),from_client_sender.clone());
	    };
	    from_client_to_persistence_receivers.push(Arc::new(Mutex::new(from_client_receiver)));
	};
    };


    from_client_to_persistence_receivers_map.insert(StoreType::Redis,from_client_to_persistence_receivers);
    let mut from_client_to_persistence_receivers = Vec::new();
   
    for dynamodb_store in config.stores.dynamodb.iter() {
	if !dynamodb_store.client_database_names.is_empty(){
	    let (from_client_sender, from_client_receiver): (mpsc::Sender<RestToPersistenceContext>, mpsc::Receiver<RestToPersistenceContext>) = mpsc::channel(10);
	    for database_name in dynamodb_store.client_database_names.iter(){
		from_client_to_persistence_senders.insert(database_name.clone(),from_client_sender.clone());
	    };
	    from_client_to_persistence_receivers.push(Arc::new(Mutex::new(from_client_receiver)));
	};
    };
    from_client_to_persistence_receivers_map.insert(StoreType::DynamoDb,from_client_to_persistence_receivers);
    
    PersistenceMemoryChannels{
	from_client_to_persistence_senders,
	from_client_to_persistence_receivers_map,
    }
    
}

pub fn create_memory_channels(config: &Swir) -> MemoryChannels {
    MemoryChannels{
	messaging_memory_channels:create_messaging_channels(config),
	persistence_memory_channels:create_persistence_channels(config)
    }
}
