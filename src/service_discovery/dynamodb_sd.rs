use crate::service_discovery::{ResolveListeners, ResolvedAddr, ServiceDiscovery};
use crate::utils::config::{AnnounceServiceDetails, ServiceDetails};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use async_trait::async_trait;
use currenttimemillis::current_time_milliseconds;
use multimap::MultiMap;
use rand::distributions::Alphanumeric;
use rand::{rngs, Rng, SeedableRng};
use rusoto_dynamodb::{DynamoDb, DynamoDbClient};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time;

fn generate_instance_name() -> String {
    rngs::SmallRng::from_entropy().sample_iter(Alphanumeric).map(char::from).take(16).collect()
}

fn get_domain(svc: &AnnounceServiceDetails) -> String {
    create_domain(&svc.service_details)
}

fn create_domain(svc: &ServiceDetails) -> String {
    return format!("_{}._{}._{}.local", svc.protocol, svc.service_name, svc.domain);
}

fn get_ip_addr() -> Option<IpAddr> {
    let mut ip_addr = None;
    for iface in get_if_addrs::get_if_addrs().unwrap() {
        let ip = &iface.ip();

        if ip.is_loopback() {
            continue;
        }
        if ip.is_multicast() {
            continue;
        }
        if let Some(addr) = ip_addr {
            if addr < *ip {
                ip_addr = Some(*ip);
            }
        } else {
            ip_addr = Some(*ip);
        };
    }
    ip_addr
}

pub struct DynamoDBServiceDiscovery {
    client: DynamoDbClient,
    port: u16,
    table: String,
    listeners: ResolveListeners,
    announced_services: Arc<RwLock<Vec<AnnounceServiceDetails>>>,
    instance_name: String,
    fqdn: String,
    ip: Option<String>,
}

impl DynamoDBServiceDiscovery {
    pub fn new(region: String, table: String, port: u16) -> Result<Self, String> {
        if let Ok(region) = rusoto_signature::Region::from_str(&region) {
            let client = rusoto_dynamodb::DynamoDbClient::new(region);
            let listeners = Arc::new(RwLock::new(MultiMap::new()));
            let instance_name = generate_instance_name();
            let fqdn = match hostname::get().map(|s| s.into_string()) {
                Ok(Ok(hst)) => hst,
                Ok(Err(_)) => {
                    warn!("Can't get hostname");
                    "EMPTY".to_string()
                }
                Err(_) => {
                    warn!("Can't get hostname");
                    "EMPTY".to_string()
                }
            };
            let announced_services = Arc::new(RwLock::new(Vec::new()));
            let ip = get_ip_addr().map(|s| s.to_string());
            let myself = Self {
                client,
                table,
                port,
                listeners,
                instance_name,
                fqdn,
                announced_services,
                ip,
            };

            myself.start();
            Ok(myself)
        } else {
            let msg = format!("Unknown region {}", region);
            warn!("{}", msg);
            Err(msg)
        }
    }

    fn start(&self) {
        let client = self.client.clone();
        let table = self.table.clone();
        let listeners = self.listeners.clone();
        let announced_services = self.announced_services.clone();
        let instance_name = self.instance_name.clone();
        let fqdn = self.fqdn.clone();
        let ip = self.ip.clone();
        let port = self.port;

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(5));
            while Option::Some(interval.tick().await).is_some() {
                Self::announce_internal(&client, &table, &announced_services, &instance_name, &fqdn, &ip, port).await;
                Self::retrieve(&client, &table, &listeners).await;
            }
        });
    }
    async fn announce_internal(
        client: &DynamoDbClient,
        table_name: &str,
        announced_services: &Arc<RwLock<Vec<AnnounceServiceDetails>>>,
        instance_name: &str,
        fqdn: &str,
        ip: &Option<String>,
        port: u16,
    ) {
        for svc in announced_services.read().await.iter() {
            let domain = get_domain(&svc);
            Self::store(&client, &table_name, &instance_name, &domain, &svc.service_details.service_name, &fqdn, ip, port).await;
        }
    }

    async fn retrieve(client: &DynamoDbClient, table_name: &str, listeners: &ResolveListeners) {
        let mut expression_attribute_values = HashMap::new();
        expression_attribute_values.insert(
            ":min_size".to_string(),
            rusoto_dynamodb::AttributeValue {
                n: Some("1".to_string()),
                ..Default::default()
            },
        );

        let cutoff_time = current_time_milliseconds() - Duration::from_secs(5).as_millis();
        expression_attribute_values.insert(
            ":cutoff_time".to_string(),
            rusoto_dynamodb::AttributeValue {
                n: Some(cutoff_time.to_string()),
                ..Default::default()
            },
        );

        let mut expression_attribute_names = HashMap::new();
        expression_attribute_names.insert(":min_size".to_string(), "N".to_string());
        expression_attribute_names.insert(":cutoff_time".to_string(), "N".to_string());

        let condition_expression = String::from("attribute_exists(last_seen_at) AND attribute_exists(service_name) AND attribute_exists(ip) AND size(fqdn) > :min_size AND size(service_name) > :min_size AND size(ip) > :min_size AND last_seen_at > :cutoff_time");

        let scan_input = rusoto_dynamodb::ScanInput {
            table_name: table_name.to_string(),
            expression_attribute_values: Some(expression_attribute_values),
            filter_expression: Some(condition_expression),
            ..Default::default()
        };

        let scan_output = client.scan(scan_input).await;

        #[derive(Debug)]
        struct ServiceDesc {
            service_name: String,
            domain: String,
            ip: String,
            port: String,
            fqdn: String,
        }

        match scan_output {
            Ok(output) => {
                if let Some(items) = output.items {
                    let services: Vec<ServiceDesc> = items
                        .iter()
                        .map(|hm| (hm.get("service_name"), hm.get("ip"), hm.get("port"), hm.get("domain"), hm.get("fqdn")))
                        .filter(|f| f.0.is_some() && f.1.is_some() && f.2.is_some() && f.3.is_some() && f.4.is_some())
                        .map(|f| (f.0.unwrap(), f.1.unwrap(), f.2.unwrap(), f.3.unwrap(), f.4.unwrap()))
                        .map(|f| (f.0.s.as_ref(), f.1.s.as_ref(), f.2.n.as_ref(), f.3.s.as_ref(), f.4.s.as_ref()))
                        .filter(|f| f.0.is_some() && f.1.is_some() && f.2.is_some() && f.3.is_some() && f.4.is_some())
                        .map(|f| (f.0.unwrap(), f.1.unwrap(), f.2.unwrap(), f.3.unwrap(), f.4.unwrap()))
                        .map(|f| ServiceDesc {
                            service_name: f.0.to_string(),
                            ip: f.1.to_string(),
                            port: f.2.to_string(),
                            domain: f.3.to_string(),
                            fqdn: f.4.to_string(),
                        })
                        .collect();
                    debug!("Services {:?}", &services);
                    let mut listeners = listeners.write().await;
                    services.iter().for_each(|svc_desc| {
                        let svc_name = &svc_desc.service_name;
                        let domain = &svc_desc.domain;
                        let ip = svc_desc.ip.parse::<IpAddr>().unwrap();
                        let port = svc_desc.port.parse::<u16>().unwrap();
                        let _sock = SocketAddr::from((ip, port));
                        let fqdn = &svc_desc.fqdn;

                        if let Some(channels) = listeners.get_vec_mut(svc_name) {
                            let mut closed_channels = vec![];
                            for (i, channel) in channels.iter_mut().enumerate() {
                                let rr = ResolvedAddr {
                                    service_name: svc_name.to_string(),
                                    domain: domain.to_string(),
                                    socket_addr: None,
                                    fqdn: Some(fqdn.to_string()),
                                    port,
                                };
                                match channel.try_send(rr) {
                                    Ok(()) => {}
                                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                                        info!("Channel full {}", svc_name);
                                    }
                                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                        warn!("Channel closed {} {}", i, svc_name);
                                        closed_channels.push(i);
                                    }
                                }
                            }
                            for i in closed_channels.iter() {
                                channels.remove(*i);
                            }
                            if channels.is_empty() {
                                listeners.remove(svc_name);
                            }
                        } else {
                            debug!("No listeners for {}", svc_name);
                        }
                    });
                }
            }
            Err(e) => {
                debug!("Error {:?}", e);
            }
        }
    }

    async fn store(client: &DynamoDbClient, table_name: &str, instance_name: &str, domain: &str, service_name: &str, fqdn: &str, ip: &Option<String>, port: u16) {
        let key_attr = rusoto_dynamodb::AttributeValue {
            s: Some(fqdn.to_string()),
            ..Default::default()
        };
        let fqdn_attr = rusoto_dynamodb::AttributeValue {
            s: Some(fqdn.to_string()),
            ..Default::default()
        };

        let instance_attr = rusoto_dynamodb::AttributeValue {
            s: Some(instance_name.to_string()),
            ..Default::default()
        };

        let domain_attr = rusoto_dynamodb::AttributeValue {
            s: Some(domain.to_string()),
            ..Default::default()
        };

        let port_attr = rusoto_dynamodb::AttributeValue {
            n: Some(port.to_string()),
            ..Default::default()
        };

        let service_name_attr = rusoto_dynamodb::AttributeValue {
            s: Some(service_name.to_string()),
            ..Default::default()
        };

        let timestamp_attr = rusoto_dynamodb::AttributeValue {
            n: Some(current_time_milliseconds().to_string()),
            ..Default::default()
        };

        let mut item = HashMap::new();
        item.insert("partition_key".to_string(), key_attr);
        item.insert("service_name".to_string(), service_name_attr);
        item.insert("domain".to_string(), domain_attr);
        item.insert("instance_name".to_string(), instance_attr);
        item.insert("last_seen_at".to_string(), timestamp_attr);
        item.insert("fqdn".to_string(), fqdn_attr);

        if let Some(addr) = ip {
            let ip_addr_attr = rusoto_dynamodb::AttributeValue {
                s: Some(addr.to_string()),
                ..Default::default()
            };
            item.insert("ip".to_string(), ip_addr_attr);
        }
        item.insert("port".to_string(), port_attr);

        let put_item_input = rusoto_dynamodb::PutItemInput {
            table_name: table_name.to_string(),
            item,
            return_values: Some("ALL_OLD".to_string()),
            ..Default::default()
        };

        trace!("Store request => {:?}", put_item_input);

        let put_item_output = client.put_item(put_item_input).await;
        match put_item_output {
            Ok(_output) => {}
            Err(e) => {
                warn!("Can't store -> {:?}", e);
            }
        }
    }
}

#[async_trait]
impl ServiceDiscovery for DynamoDBServiceDiscovery {
    async fn resolve(&self, svc: &ServiceDetails, sender: mpsc::Sender<ResolvedAddr>) {
        let domain = create_domain(&svc);
        debug!("resolver: resolving domain {}", domain);
        let mut listeners = self.listeners.write().await;
        listeners.insert(svc.service_name.clone(), sender);
    }

    async fn announce(&self, svc: &AnnounceServiceDetails) {
        let domain = get_domain(&svc);
        debug!("resolver: announcing service {}", domain);
        let mut services = self.announced_services.write().await;
        services.push(svc.clone());
    }
}
