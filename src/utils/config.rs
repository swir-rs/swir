use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Kafka {
    pub brokers: Vec<String>,
    pub consumer_topics: Vec<String>,
    pub consumer_groups: Vec<String>,
    pub producer_topics: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Nats {
    pub brokers: Vec<String>,
    pub consumer_topics: Vec<String>,
    pub consumer_groups: Vec<String>,
    pub producer_topics: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Messaging {
    pub kafka: Kafka,
    pub nats: Nats,
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
    pub messaging: Messaging,
}

impl Swir {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();
        s.merge(File::with_name("swir.yaml"))?;
        let s = s.try_into();
        println!("SWIR Config : {:?}", s);
        s
    }
}
