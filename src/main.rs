//#![Deny(warnings)]
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

extern crate custom_error;

mod frontend_handlers;
mod messaging_handlers;
mod persistence_handlers;
mod si_handlers;
mod utils;
use si_handlers::si_http_handler;
mod service_discovery;

pub mod swir_grpc_internal_api {
    tonic::include_proto!("swir_internal");
}
pub mod swir_common {
    tonic::include_proto!("swir_common");
}

pub mod swir_grpc_api {
    tonic::include_proto!("swir_public");
}

use std::sync::Arc;

use frontend_handlers::http_handler::{client_handler, handler};
use frontend_handlers::{grpc_handler, grpc_internal_handler};
use hyper::service::{make_service_fn, service_fn};

use futures_util::stream::StreamExt;
use hyper::{Body, Request, Server};
use utils::pki_utils::{load_certs, load_private_key};

use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::utils::config::*;

#[tokio::main(core_threads = 8)]
async fn main() {    
    env_logger::builder().format_timestamp_nanos().init();
    let swir_config = Swir::new();

    let mc: MemoryChannels = utils::config::create_memory_channels(&swir_config);
    let mmc = mc.messaging_memory_channels;
    let pmc = mc.persistence_memory_channels;
    let simc = mc.si_memory_channels;

    let ip = swir_config.ip.clone();
    let https_port: u16 = swir_config.https_port;
    let http_port: u16 = swir_config.http_port;
    let grpc_port: u16 = swir_config.grpc_port;
    let internal_grpc_port: u16 = swir_config.internal_grpc_port;
    let client_executable = swir_config.client_executable.clone();

    let http_tls_certificate = swir_config.tls_certificate.clone();
    let http_tls_key = swir_config.tls_private_key.clone();

    let https_addr = std::net::SocketAddr::new(ip.parse().unwrap(), https_port);
    let http_addr = std::net::SocketAddr::new(ip.parse().unwrap(), http_port);
    let grpc_addr = std::net::SocketAddr::new(ip.parse().unwrap(), grpc_port);
    let internal_grpc_addr = std::net::SocketAddr::new(ip.parse().unwrap(), internal_grpc_port);
    let certs = load_certs(http_tls_certificate).unwrap();
    // Load private key.
    let key = load_private_key(http_tls_key).unwrap();

    let to_client_sender_for_rest = mmc.to_client_sender_for_rest.clone();

    let to_si_http_client = mmc.to_si_http_client.clone();

    let from_client_to_messaging_sender = mmc.from_client_to_messaging_sender.clone();
    let from_client_to_persistence_senders = pmc.from_client_to_persistence_senders.clone();

    let client_sender_for_http = simc.client_sender.clone();
    let client_sender_for_private_http = client_sender_for_http.clone();
    let client_sender_for_https = simc.client_sender.clone();
    let http_service = make_service_fn(move |_| {
        let from_client_to_messaging_sender = from_client_to_messaging_sender.clone();
        let from_client_to_persistence_senders = from_client_to_persistence_senders.clone();
        let to_client_sender_for_rest = to_client_sender_for_rest.clone();
        let client_sender_for_http = client_sender_for_http.to_owned();
        async move {
            let from_client_to_messaging_sender = from_client_to_messaging_sender.clone();
            let to_client_sender_for_rest = to_client_sender_for_rest;
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                handler(
                    req,
                    from_client_to_messaging_sender.clone(),
                    to_client_sender_for_rest.clone(),
                    from_client_to_persistence_senders.clone(),
                    client_sender_for_http.to_owned(),
                )
            }))
        }
    });

    let to_client_sender_for_rest = mmc.to_client_sender_for_rest.clone();
    let from_client_to_messaging_sender = mmc.from_client_to_messaging_sender.clone();
    let from_client_to_persistence_senders = pmc.from_client_to_persistence_senders.clone();

    let https_service = make_service_fn(move |_| {
        let from_client_to_messaging_sender = from_client_to_messaging_sender.clone();
        let to_client_sender_for_rest = to_client_sender_for_rest.clone();
        let from_client_to_persistence_senders = from_client_to_persistence_senders.clone();
        let client_sender_for_https = client_sender_for_https.to_owned();
        async move {
            let from_client_to_messaging_sender = from_client_to_messaging_sender.clone();
            let to_client_sender_for_rest = to_client_sender_for_rest;
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                handler(
                    req,
                    from_client_to_messaging_sender.clone(),
                    to_client_sender_for_rest.clone(),
                    from_client_to_persistence_senders.clone(),
                    client_sender_for_https.to_owned(),
                )
            }))
        }
    });

    let from_client_to_persistence_senders = pmc.from_client_to_persistence_senders.clone();
    let from_client_to_messaging_sender = mmc.from_client_to_messaging_sender.clone();
    let to_client_receiver_for_rest = mmc.to_client_receiver_for_rest.clone();
    let to_client_receiver_for_grpc = mmc.to_client_receiver_for_grpc.clone();

    let mut tasks = vec![];
    let config = swir_config.clone();
    let messaging = tokio::spawn(async move {
        messaging_handlers::configure_broker(config.pubsub, mmc).await;
    });
    tasks.push(messaging);

    let config = swir_config.clone();
    let persistence = tokio::spawn(async move {
        persistence_handlers::configure_stores(config.stores, pmc.from_client_to_persistence_receivers_map).await;
    });
    tasks.push(persistence);

    let config = swir_config.clone();
    let receiver = simc.receiver;

    let config_si = config.services.clone();
    let si = tokio::spawn(async move {
        if let Some(services) = config_si {
            match services.resolver.resolver_type {
                ResolverType::MDNS => {
                    if let Ok(resolver) = service_discovery::mdns_sd::MDNSServiceDiscovery::new(internal_grpc_port) {
                        si_handlers::ServiceInvocationService::new().start(services, &resolver, receiver, to_si_http_client).await;
                    } else {
                        warn!("Problem with resolver");
                    };
                }
                ResolverType::DynamoDb => {
                    if let Some(resolver_config) = &services.resolver.resolver_config {
                        let region = resolver_config.get("region");
                        let table = resolver_config.get("table");
                        if let (Some(r), Some(t)) = (region, table) {
                            if let Ok(resolver) = service_discovery::dynamodb_sd::DynamoDBServiceDiscovery::new(r.to_string(), t.to_string(), internal_grpc_port) {
                                si_handlers::ServiceInvocationService::new().start(services, &resolver, receiver, to_si_http_client).await;
                            } else {
                                warn!("Problem with resolver");
                            };
                        } else {
                            warn!("Problem with resolver: Invalid resolver config");
                        }
                    }
                }
            }
        }
    });

    tasks.push(si);
    let maybe_si_pi_config = config.services.clone();
    let si_private_interface = tokio::spawn(async move {
        if let Some(services) = maybe_si_pi_config {
            if let Some(private_http_socket) = services.private_http_socket {
                info!("Private invocation service enabled at {}", private_http_socket);
                let http_service = make_service_fn(move |_| {
                    let client_sender_for_http = client_sender_for_private_http.to_owned();
                    async move { Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| si_http_handler::handler(req, client_sender_for_http.to_owned()))) }
                });
                if let Err(e) = Server::bind(&private_http_socket).serve(http_service).await {
                    warn!("Problem starting HTTP interface {:?}", e);
                }
            };
        }
    });

    tasks.push(si_private_interface);

    let http_client_interface = tokio::spawn(async move {
        let res = Server::bind(&http_addr).serve(http_service).await;
        if let Err(e) = res {
            warn!("Problem starting HTTP interface {:?}", e);
        }
    });
    tasks.push(http_client_interface);

    let https_client_interface = tokio::spawn(async move {
        let mut config = rustls::ServerConfig::new(rustls::NoClientAuth::new());
        config.set_single_cert(certs, key).expect("invalid key or certificate");
        let tls_acceptor = TlsAcceptor::from(Arc::new(config));
        let _arc_acceptor = Arc::new(tls_acceptor);
        let mut listener = TcpListener::bind(&https_addr).await.unwrap();
        let incoming = listener.incoming();

        let incoming = hyper::server::accept::from_stream(incoming.filter_map(|socket| async {
            match socket {
                Ok(stream) => match _arc_acceptor.clone().accept(stream).await {
                    Ok(val) => Some(Ok::<_, hyper::Error>(val)),
                    Err(e) => {
                        println!("TLS error: {}", e);
                        None
                    }
                },
                Err(e) => {
                    println!("TCP socket error: {}", e);
                    None
                }
            }
        }));
        let res = Server::builder(incoming).serve(https_service).await;
        if let Err(e) = res {
            warn!("Problem starting HTTPs interface {:?}", e);
        }
    });

    tasks.push(https_client_interface);

    let http_client = tokio::spawn(async move { client_handler(to_client_receiver_for_rest.clone()).await });
    tasks.push(http_client);

    let client_sender_for_public = simc.client_sender.clone();
    let client_sender_for_internal = simc.client_sender.clone();

    let grpc_client_interface = tokio::spawn(async move {
        let pub_sub_handler = grpc_handler::SwirPubSubApi::new(from_client_to_messaging_sender.clone(), to_client_receiver_for_grpc.clone());
        let persistence_handler = grpc_handler::SwirPersistenceApi::new(from_client_to_persistence_senders);

        let pub_sub_svc = swir_grpc_api::pub_sub_api_server::PubSubApiServer::new(pub_sub_handler);
        let persistence_svc = swir_grpc_api::persistence_api_server::PersistenceApiServer::new(persistence_handler);

        let service_invocation_handler = grpc_handler::SwirServiceInvocationApi::new(client_sender_for_public);
        let service_invocation_svc = swir_grpc_api::service_invocation_api_server::ServiceInvocationApiServer::new(service_invocation_handler);

        let grpc = tonic::transport::Server::builder()
            .add_service(pub_sub_svc)
            .add_service(persistence_svc)
            .add_service(service_invocation_svc)
            .serve(grpc_addr);

        let res = grpc.await;
        if let Err(e) = res {
            warn!("Problem starting gRPC interface {:?}", e);
        }
    });
    

    tasks.push(grpc_client_interface);
    let si_config = config.services.clone();
    let grpc_internal_interface = tokio::spawn(async move {
	if let Some(services) = si_config {
	    let tls_config = services.tls_config;
            let service_invocation_handler = grpc_internal_handler::SwirServiceInvocationDiscoveryApi::new(client_sender_for_internal);
            let service_invocation_svc = swir_grpc_internal_api::service_invocation_discovery_api_server::ServiceInvocationDiscoveryApiServer::new(service_invocation_handler);
	    
	    let cert = tokio::fs::read(tls_config.server_cert).await.unwrap();
	    let key = tokio::fs::read(tls_config.server_key).await.unwrap();
	    let server_identity = tonic::transport::Identity::from_pem(cert, key);	
	    let client_ca_cert = tokio::fs::read(tls_config.client_ca_cert).await.unwrap();
	    let client_ca_cert = tonic::transport::Certificate::from_pem(client_ca_cert);

	    let tls = tonic::transport::ServerTlsConfig::new().identity(server_identity).client_ca_root(client_ca_cert);
			    
            let grpc = tonic::transport::Server::builder().tls_config(tls).add_service(service_invocation_svc).serve(internal_grpc_addr);
	    
            let res = grpc.await;
            if let Err(e) = res {
		warn!("Problem starting gRPC interface {:?}", e);
            }
	}
    });
    tasks.push(grpc_internal_interface);

    if let Some(command) = client_executable {
        utils::command_utils::run_java_command(command);
    }
    futures::future::join_all(tasks).await;
}
