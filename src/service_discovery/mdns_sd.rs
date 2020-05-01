use crate::service_discovery::{ResolvedAddr, ServiceDiscovery};
use crate::utils::config::{AnnounceServiceDetails, ServiceDetails};
use async_trait::async_trait;
use rand::distributions::Alphanumeric;
use rand::{rngs, Rng, SeedableRng};
use std::net::SocketAddr;
use tokio::sync::mpsc;

fn parse_service_name(service: &str) -> Option<(String, String)> {
    let ind = service.find('.');
    if let Some(i) = ind {
        let service_type = String::from(&service[(i + 1)..]);
        if service_type.ends_with("._swir.local") {
            let ending_len = "._swir.local".len();
            let service_type_len = service_type.len();
            let pos = service_type_len - ending_len;
            let prot_and_service = String::from(&service_type[..pos]).replacen('_', " ", 1).trim().to_string();
            if let Some(i) = prot_and_service.find('.') {
                let service = String::from(&prot_and_service[(i + 1)..]).replacen('_', " ", 1).trim().to_string();
                Some((service, service_type))
            } else {
                Some((prot_and_service, service_type))
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub struct MDNSServiceDiscovery {
    responder: mdns_responder::Responder,
    port: u16,
}

impl MDNSServiceDiscovery {
    pub fn new(port: u16) -> Result<Self, String> {
        let maybe_responder = mdns_responder::Responder::new();
        if let Ok(responder) = maybe_responder {
            responder.start();
            Ok(Self { responder, port })
        } else {
            Err("Can't start responder".to_string())
        }
    }
    fn generate_instance_name(&self) -> String {
        rngs::SmallRng::from_entropy().sample_iter(Alphanumeric).take(16).collect()
    }

    fn get_service_domain_and_name(&self, svc: &AnnounceServiceDetails) -> (String, String) {
        let domain = self.create_domain(&svc.service_details);
        let name = self.generate_instance_name();
        (name, domain)
    }

    fn create_domain(&self, svc: &ServiceDetails) -> String {
        return format!("_{}._{}._{}.local", svc.protocol, svc.service_name, svc.domain);
    }
}

#[async_trait]
impl ServiceDiscovery for MDNSServiceDiscovery {
    async fn resolve(&self, svc: &ServiceDetails, mut sender: mpsc::Sender<ResolvedAddr>) {
        let domain = self.create_domain(&svc);
        debug!("resolver: resolving domain {}", domain);

        let f = move |domain: String, socket_addr: SocketAddr| {
            if let Some((service_name, _svc_domain)) = parse_service_name(&domain) {
                let ra = ResolvedAddr {
                    service_name,
                    domain,
                    socket_addr: Some(socket_addr),
                    fqdn: None,
                    port: socket_addr.port(),
                };
                let res = sender.try_send(ra);
                match res {
                    Ok(()) => Ok(()),
                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => Ok(()),
                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => Err(()),
                }
            } else {
                Ok(())
            }
        };

        self.responder.resolve(domain.to_owned(), Box::new(f)).await;
    }

    async fn announce(&self, svc: &AnnounceServiceDetails) {
        let (mdns_name, mdns_domain) = self.get_service_domain_and_name(&svc);
        debug!("resolver: announcing service {} {}", mdns_name, mdns_domain);
        let _svc = self.responder.register(mdns_domain.to_owned(), mdns_name.to_owned(), self.port, &["path=/"]).await;
    }
}
