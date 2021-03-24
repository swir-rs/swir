use config::{Config, File};

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::utils::structs::{BackendToRestContext, RestToMessagingContext, RestToPersistenceContext, RestToSIContext};

#[derive(Hash, PartialEq, Eq)]
pub enum StoreType {
    Redis,
    DynamoDb,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProducerTopic {
    pub producer_topic: String,
    pub client_topic: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConsumerTopic {
    pub consumer_topic: String,
    pub consumer_group: String,
    pub client_topic: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Kafka {
    pub brokers: Vec<String>,
    pub producer_topics: Vec<ProducerTopic>,
    pub consumer_topics: Vec<ConsumerTopic>,
}

pub trait ClientTopicsConfiguration {
    fn get_producer_topic_for_client_topic(&self, client_topic: &str) -> Option<String>;
    fn get_consumer_topic_for_client_topic(&self, client_topic: &str) -> Option<String>;
}

pub trait ClientToBackendDatabaseResolver {
    fn get_backend_table_name_for_client_table_name(&self, client_name: &str) -> Option<String> {
        for t in self.get_tables().iter() {
            if t.client_name.eq(client_name) {
                return Some(t.table_name.clone());
            }
        }
        None
    }
    fn get_tables(&self) -> &Vec<TableDescription>;
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

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
pub struct TableDescription {
    pub table_name: String,
    pub client_name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Redis {
    pub nodes: Vec<String>,
    pub tables: Vec<TableDescription>,
}

impl ClientToBackendDatabaseResolver for Redis {
    fn get_tables(&self) -> &Vec<TableDescription> {
        &self.tables
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DynamoDb {
    pub region: String,
    pub tables: Vec<TableDescription>,
}

impl ClientToBackendDatabaseResolver for DynamoDb {
    fn get_tables(&self) -> &Vec<TableDescription> {
        &self.tables
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PubSub {
    pub kafka: Vec<Kafka>,
    pub nats: Vec<Nats>,
    pub aws_kinesis: Vec<AwsKinesis>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Stores {
    pub redis: Vec<Redis>,
    pub dynamodb: Vec<DynamoDb>,
}

#[derive(Debug, Clone)]
pub struct MemoryChannelEndpoint<T1> {
    pub from_client_receiver: Arc<Mutex<mpsc::Receiver<T1>>>,
}

pub struct MessagingMemoryChannels {
    pub kafka_memory_channels: Vec<MemoryChannelEndpoint<RestToMessagingContext>>,
    pub nats_memory_channels: Vec<MemoryChannelEndpoint<RestToMessagingContext>>,
    pub aws_kinesis_memory_channels: Vec<MemoryChannelEndpoint<RestToMessagingContext>>,
    pub to_client_sender_for_rest: mpsc::Sender<BackendToRestContext>,
    pub to_client_receiver_for_rest: Arc<Mutex<mpsc::Receiver<BackendToRestContext>>>,
    pub to_client_sender_for_grpc: mpsc::Sender<BackendToRestContext>,
    pub to_client_receiver_for_grpc: Arc<Mutex<mpsc::Receiver<BackendToRestContext>>>,
    pub from_client_to_messaging_sender: HashMap<String, mpsc::Sender<RestToMessagingContext>>,
}

pub struct PersistenceMemoryChannels {
    pub from_client_to_persistence_senders: HashMap<String, mpsc::Sender<RestToPersistenceContext>>,
    pub from_client_to_persistence_receivers_map: HashMap<StoreType, Vec<Arc<Mutex<mpsc::Receiver<RestToPersistenceContext>>>>>,
}

pub struct MemoryChannels {
    pub messaging_memory_channels: MessagingMemoryChannels,
    pub persistence_memory_channels: PersistenceMemoryChannels,
    pub si_memory_channels: SIMemoryChannel,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServiceDetails {
    pub service_name: String,
    pub domain: String,
    pub protocol: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AnnounceServiceDetails {
    pub service_details: ServiceDetails,
    pub client_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ResolvedServiceDetails {
    pub addr: std::net::SocketAddr,
}

#[derive(Debug, Deserialize, Clone)]
pub enum ResolverType {
    MDNS,
    DynamoDb,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Resolver {
    pub resolver_type: ResolverType,
    pub resolver_config: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TlsConfig {
    pub client_ca_cert: String,
    pub client_cert: String,
    pub client_key: String,
    pub server_ca_cert: String,
    pub server_cert: String,
    pub server_key: String,
    pub domain_name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerTlsConfig {
    pub private_key: String,
    pub certificate: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Services {
    pub private_http_socket: Option<std::net::SocketAddr>,
    pub resolver: Resolver,
    pub tls_config: TlsConfig,
    pub resolve_services: Vec<ServiceDetails>,
    pub announce_services: Vec<AnnounceServiceDetails>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OpenTelemetry {
    pub collector_address: String,
    pub collector_port: u16,
    pub service_name: String,
    pub metric_window: Option<u64>
}

#[derive(Debug, Deserialize, Clone)]
pub struct TracingConfig {
    pub open_telemetry: Option<OpenTelemetry>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClientConfig {
    pub ip: String,
    pub http_port: Option<u16>,
    pub grpc_port: Option<u16>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Swir {
    pub tracing: Option<TracingConfig>,
    pub ip: String,
    pub http_port: u16,
    pub grpc_port: u16,
    pub internal_grpc_port: u16,
    pub client_config: Option<ClientConfig>,
    pub tls_config: Option<ServerTlsConfig>,
    pub client_executable: Option<String>,
    pub pubsub: PubSub,
    pub stores: Stores,
    pub services: Option<Services>,
}

impl Swir {
    pub fn new() -> Box<Swir> {
        let mut s = Config::new();
        let env_conf = config::Environment::with_prefix("SWIR");
        s.merge(env_conf).unwrap();
        match s.get_str("config_file") {
            Ok(config_file) => {
                debug!("Trying config file at : {}", config_file);
                s.merge(File::with_name(&config_file)).unwrap();
            }
            Err(e) => {
                debug!("Trying default config file ./swir.yaml {:?}", e);
                s.merge(File::with_name("swir.yaml")).unwrap();
            }
        }

        debug!("{:?}", s);
        let s = s.try_into().unwrap();
        info!("SWIR Config : {:?}", s);
        Box::new(s)
    }
}




// #[async_trait]
// trait Sender<T:Send+Sync+'static>{
//     async fn send(&self, value: T) -> Result<(), mpsc::error::SendError<T>>;
//     async fn closed(&self);
//     fn try_send(&self, message: T) -> Result<(), mpsc::error::TrySendError<T>>;
//     async fn send_timeout( &self, value: T,timeout: core::time::Duration)-> Result<(), mpsc::error::SendTimeoutError<T>>;
 
// }

// #[async_trait]
// trait Receiver<T:Send+Sync+'static>{
//     async fn recv(&mut self) -> Option<T>;
//     fn blocking_recv(&mut self) -> Option<T>;
//     fn close(&mut self);
//     fn poll_recv(&mut self, cx: &mut core::task::Context<'_>) -> core::task::Poll<Option<T>>;
// }

// struct MySender<T>(mpsc::Sender<T>);
// struct MyReceiver<T>(mpsc::Receiver<T>);

// #[async_trait]
// impl<T:Send+Sync+'static> Sender<T> for MySender<T>{
//     async fn send(&self, value: T) -> Result<(), mpsc::error::SendError<T>>{
// 	self.0.send(value).await
//     }
//     async fn closed(&self){
// 	self.0.closed().await
//     }
//     fn try_send(&self, message: T) -> Result<(), mpsc::error::TrySendError<T>>{
//     	self.0.try_send(message)
//     }
//     async fn send_timeout( &self, value: T,timeout: core::time::Duration)-> Result<(), mpsc::error::SendTimeoutError<T>>{
//     	self.0.send_timeout(value, timeout).await
//     }

// }

// #[async_trait]
// impl<T:Send+Sync+'static> Receiver<T> for MyReceiver<T>{
//     async fn recv(&mut self) -> Option<T>{
// 	self.0.recv().await
//     }
//     fn blocking_recv(&mut self) -> Option<T>{
// 	self.0.blocking_recv()
//     }
//     fn close(&mut self){
// 	self.0.close()
//     }
//     fn poll_recv(&mut self, cx: &mut core::task::Context<'_>) -> core::task::Poll<Option<T>>{
// 	self.0.poll_recv(cx)
//     }  
// }


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

fn create_messaging_channels(config: &Swir) -> MessagingMemoryChannels {
    let mut subscribe_topics = vec![];
    let mut publish_topics = vec![];

    let (to_client_sender_for_grpc, to_client_receiver_for_grpc): (mpsc::Sender<BackendToRestContext>, mpsc::Receiver<BackendToRestContext>) = mpsc::channel(20000);
    let (to_client_sender_for_rest, to_client_receiver_for_rest): (mpsc::Sender<BackendToRestContext>, mpsc::Receiver<BackendToRestContext>) = mpsc::channel(20000);
    

    let to_client_receiver_for_rest = Arc::new(Mutex::new(to_client_receiver_for_rest));
    let to_client_sender_for_grpc = to_client_sender_for_grpc;
    let to_client_receiver_for_grpc = Arc::new(Mutex::new(to_client_receiver_for_grpc));

    let mut kafka_memory_channels = vec![];
    let mut from_client_to_messaging_sender = HashMap::new();

    for kafka_channels in config.pubsub.kafka.iter() {
        let (from_client_sender, from_client_receiver): (mpsc::Sender<RestToMessagingContext>, mpsc::Receiver<RestToMessagingContext>) = mpsc::channel(20000);
        let mme = MemoryChannelEndpoint {
            from_client_receiver: Arc::new(Mutex::new(from_client_receiver)),
        };

        kafka_memory_channels.push(mme);
        for producer_topic in kafka_channels.producer_topics.iter() {
            from_client_to_messaging_sender.insert(producer_topic.client_topic.clone(), from_client_sender.clone());
            publish_topics.push(producer_topic.client_topic.clone());
        }

        for consumer_topic in kafka_channels.consumer_topics.iter() {
            from_client_to_messaging_sender.insert(consumer_topic.client_topic.clone(), from_client_sender.clone());
            subscribe_topics.push(consumer_topic.client_topic.clone());
        }
    }

    let mut aws_kinesis_memory_channels = vec![];
    for aws_kinesis_channels in config.pubsub.aws_kinesis.iter() {
        let (from_client_sender, from_client_receiver): (mpsc::Sender<RestToMessagingContext>, mpsc::Receiver<RestToMessagingContext>) = mpsc::channel(20000);
        let mme = MemoryChannelEndpoint {
            from_client_receiver: Arc::new(Mutex::new(from_client_receiver)),
        };

        aws_kinesis_memory_channels.push(mme);
        for producer_topic in aws_kinesis_channels.producer_topics.iter() {
            from_client_to_messaging_sender.insert(producer_topic.client_topic.clone(), from_client_sender.clone());
            publish_topics.push(producer_topic.client_topic.clone());
        }

        for consumer_topic in aws_kinesis_channels.consumer_topics.iter() {
            from_client_to_messaging_sender.insert(consumer_topic.client_topic.clone(), from_client_sender.clone());
            subscribe_topics.push(consumer_topic.client_topic.clone());
        }
    }

    let mut nats_memory_channels = vec![];
    #[cfg(feature = "with_nats")]
    {
        for nats_channels in config.pubsub.nats.iter() {
            let (from_client_sender, from_client_receiver): (mpsc::Sender<RestToMessagingContext>, mpsc::Receiver<RestToMessagingContext>) = mpsc::channel(20000);

            let mme = MemoryChannelEndpoint {
                from_client_receiver: Arc::new(Mutex::new(from_client_receiver)),
            };
            nats_memory_channels.push(mme);
            for producer_topic in nats_channels.producer_topics.iter() {
                from_client_to_messaging_sender.insert(producer_topic.client_topic.clone(), from_client_sender.clone());
                publish_topics.push(producer_topic.client_topic.clone());
            }

            for consumer_topic in nats_channels.consumer_topics.iter() {
                from_client_to_messaging_sender.insert(consumer_topic.client_topic.clone(), from_client_sender.clone());
                subscribe_topics.push(consumer_topic.client_topic.clone());
            }
        }
    }
    MessagingMemoryChannels {
        kafka_memory_channels,
        nats_memory_channels,
        aws_kinesis_memory_channels,
        to_client_sender_for_rest,
        to_client_receiver_for_rest,
        to_client_receiver_for_grpc,
        to_client_sender_for_grpc,
        from_client_to_messaging_sender,
    }
}

fn create_persistence_channels(config: &Swir) -> PersistenceMemoryChannels {
    let mut from_client_to_persistence_senders = HashMap::new();
    let mut from_client_to_persistence_receivers_map = HashMap::new();
    let mut from_client_to_persistence_receivers = Vec::new();

    for redis_store in config.stores.redis.iter() {
        if !redis_store.tables.is_empty() {
            let (from_client_sender, from_client_receiver): (mpsc::Sender<RestToPersistenceContext>, mpsc::Receiver<RestToPersistenceContext>) = mpsc::channel(10);
            for database_desc in redis_store.tables.iter() {
                from_client_to_persistence_senders.insert(database_desc.client_name.clone(), from_client_sender.clone());
            }
            from_client_to_persistence_receivers.push(Arc::new(Mutex::new(from_client_receiver)));
        };
    }

    from_client_to_persistence_receivers_map.insert(StoreType::Redis, from_client_to_persistence_receivers);
    let mut from_client_to_persistence_receivers = Vec::new();

    for dynamodb_store in config.stores.dynamodb.iter() {
        if !dynamodb_store.tables.is_empty() {
            let (from_client_sender, from_client_receiver): (mpsc::Sender<RestToPersistenceContext>, mpsc::Receiver<RestToPersistenceContext>) = mpsc::channel(10);
            for database_desc in dynamodb_store.tables.iter() {
                from_client_to_persistence_senders.insert(database_desc.client_name.clone(), from_client_sender.clone());
            }
            from_client_to_persistence_receivers.push(Arc::new(Mutex::new(from_client_receiver)));
        };
    }
    from_client_to_persistence_receivers_map.insert(StoreType::DynamoDb, from_client_to_persistence_receivers);

    PersistenceMemoryChannels {
        from_client_to_persistence_senders,
        from_client_to_persistence_receivers_map,
    }
}

pub struct SIMemoryChannel {
    pub client_sender: mpsc::Sender<RestToSIContext>,
    pub internal_sender: mpsc::Sender<RestToSIContext>,
    pub receiver: Arc<Mutex<mpsc::Receiver<RestToSIContext>>>,
}

fn create_service_invocation_channels(_config: &Swir) -> SIMemoryChannel {
    let (from_client_sender, from_client_receiver): (mpsc::Sender<RestToSIContext>, mpsc::Receiver<RestToSIContext>) = mpsc::channel(10);
    SIMemoryChannel {
        client_sender: from_client_sender.clone(),
        internal_sender: from_client_sender,
        receiver: Arc::new(Mutex::new(from_client_receiver)),
    }
}

pub fn create_memory_channels(config: &Swir) -> MemoryChannels {
    MemoryChannels {
        messaging_memory_channels: create_messaging_channels(config),
        persistence_memory_channels: create_persistence_channels(config),
        si_memory_channels: create_service_invocation_channels(config),
    }
}
