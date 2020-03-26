//#![Deny(warnings)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use std::{
    sync::Arc,
};

use hyper::service::{make_service_fn, service_fn};
use frontend_handlers::http_handler::{client_handler,handler};

use utils::pki_utils::{load_certs, load_private_key};
use hyper::{
    Body, Request, Server,
};
use futures_util::stream::StreamExt;

use tokio_rustls::TlsAcceptor;
use tokio::net::TcpListener;

use crate::utils::config::MemoryChannel;
use frontend_handlers::grpc_handler;

mod frontend_handlers;
mod backend_handlers;
mod utils;



#[tokio::main(core_threads = 8)]
async fn main() {
    color_backtrace::install();
    env_logger::builder().format_timestamp_nanos().init();
    let swir_config = utils::config::Swir::new();

    let mc: MemoryChannel = utils::config::create_memory_channels(&swir_config);

    let client_ip = swir_config.client_ip.clone();
    let client_https_port: u16 = swir_config.client_https_port;
    let client_http_port: u16 = swir_config.client_http_port;
    let client_grpc_port: u16 = swir_config.client_grpc_port;
    let client_executable = swir_config.client_executable.clone();

    let http_tls_certificate = swir_config.client_tls_certificate.clone();
    let http_tls_key = swir_config.client_tls_private_key.clone();

    let client_https_addr = std::net::SocketAddr::new(client_ip.parse().unwrap(), client_https_port);
    let client_http_addr = std::net::SocketAddr::new(client_ip.parse().unwrap(), client_http_port);
    let client_grpc_addr = std::net::SocketAddr::new(client_ip.parse().unwrap(), client_grpc_port);
    let certs = load_certs(http_tls_certificate).unwrap();
    // Load private key.
    let key = load_private_key(http_tls_key).unwrap();

    let mut config = rustls::ServerConfig::new(rustls::NoClientAuth::new());
    config.set_single_cert(certs, key).expect("invalid key or certificate");
    let tls_acceptor = TlsAcceptor::from(Arc::new(config));
    let _arc_acceptor = Arc::new(tls_acceptor);
    let mut listener = TcpListener::bind(&client_https_addr).await.unwrap();
    let incoming = listener.incoming();

    let incoming=  hyper::server::accept::from_stream(incoming.filter_map(|socket| {
            async {
                match socket {
                    Ok(stream) => {
                        match _arc_acceptor.clone().accept(stream).await {
                            Ok(val) => Some(Ok::<_, hyper::Error>(val)),
                            Err(e) => {
                                println!("TLS error: {}", e);
                                None
                            }
                        }
                    },
                    Err(e) => {
                        println!("TCP socket error: {}", e);
                        None
                    }
                }
            }
        }));
    
    let to_client_sender_for_rest = mc.to_client_sender_for_rest.clone();
    let from_client_to_messaging_sender = mc.from_client_to_messaging_sender.clone();

    
    let http_service = make_service_fn(move |_| {
        let from_client_to_messaging_sender = from_client_to_messaging_sender.clone();
	let to_client_sender_for_rest = to_client_sender_for_rest.clone();
        async move {
            let from_client_to_messaging_sender = from_client_to_messaging_sender.clone();
	    let to_client_sender_for_rest = to_client_sender_for_rest;
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| handler(req, from_client_to_messaging_sender.clone(),to_client_sender_for_rest.clone())))
        }
    });

    let to_client_sender_for_rest = mc.to_client_sender_for_rest.clone();
    let from_client_to_messaging_sender = mc.from_client_to_messaging_sender.clone();
    
    let https_service = make_service_fn(move |_| {
        let from_client_to_messaging_sender = from_client_to_messaging_sender.clone();
	let to_client_sender_for_rest = to_client_sender_for_rest.clone();
        async move {
            let from_client_to_messaging_sender = from_client_to_messaging_sender.clone();
	    let to_client_sender_for_rest = to_client_sender_for_rest;
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| handler(req, from_client_to_messaging_sender.clone(),to_client_sender_for_rest.clone())))
        }
    });

    let from_client_to_messaging_sender = mc.from_client_to_messaging_sender.clone();
    let to_client_receiver_for_rest = mc.to_client_receiver_for_rest.clone();
    let to_client_receiver_for_grpc = mc.to_client_receiver_for_grpc.clone();
    let from_client_to_persistence_sender = mc.from_client_to_persistence_sender.clone();
    let broker = async {
        backend_handlers::configure_broker(swir_config.channels,  mc).await;
    };

    let server = Server::bind(&client_http_addr).serve(http_service);
    let tls_server = Server::builder(incoming).serve(https_service);
    let client = async { client_handler(to_client_receiver_for_rest.clone()).await };
    let pub_sub_handler = grpc_handler::SwirPubSubApi::new(from_client_to_messaging_sender.clone(), to_client_receiver_for_grpc.clone());
    let persistence_handler = grpc_handler::SwirPersistenceApi::new(from_client_to_persistence_sender);
    
    let pub_sub_svc = grpc_handler::swir_grpc_api::pub_sub_api_server::PubSubApiServer::new(pub_sub_handler);
    let persistence_svc = grpc_handler::swir_grpc_api::persistence_api_server::PersistenceApiServer::new(persistence_handler);
    let grpc = tonic::transport::Server::builder()
	.add_service(pub_sub_svc)
	.add_service(persistence_svc)
	.serve(client_grpc_addr);

    if let Some(command) = client_executable {
        utils::command_utils::run_java_command(command);
    }

    let (_r1, _r2, _r3, _r4, _r5) = futures::join!(tls_server, server, client, broker, grpc);
}
