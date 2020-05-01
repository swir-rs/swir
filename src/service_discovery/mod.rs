use crate::utils::config::{AnnounceServiceDetails, ServiceDetails};
use async_trait::async_trait;
use std::net::SocketAddr;

use multimap::MultiMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

pub mod dynamodb_sd;
pub mod mdns_sd;

#[derive(Debug)]
pub struct ResolvedAddr {
    pub service_name: String,
    pub fqdn: Option<String>,
    pub socket_addr: Option<SocketAddr>,
    pub domain: String,
    pub port: u16,
}

#[async_trait]
pub trait ServiceDiscovery {
    async fn resolve(&self, svc: &ServiceDetails, sender: mpsc::Sender<ResolvedAddr>);
    async fn announce(&self, svc: &AnnounceServiceDetails);
}

type ResolveListeners = Arc<RwLock<MultiMap<String, mpsc::Sender<ResolvedAddr>>>>;
