//#![Deny(warnings)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use std::io::{Error as StdError, ErrorKind};
use std::{
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures_core::Stream;
use futures_util::{ready, TryStreamExt};
use hyper::service::{make_service_fn, service_fn};
use frontend_handlers::http_handler::{client_handler,handler};

use utils::pki_utils::{load_certs, load_private_key};
use hyper::{
    server::{accept::Accept, conn},
    Body, Request, Server,
};

use tokio_rustls::TlsAcceptor;
use crate::utils::config::MemoryChannel;
use boxio::BoxedIo;
use frontend_handlers::grpc_handler;
mod boxio;
mod frontend_handlers;
mod backend_handlers;
mod utils;




#[derive(Debug)]
struct TcpIncoming {
    inner: conn::AddrIncoming,
}

impl TcpIncoming {
    fn bind(addr: SocketAddr) -> Result<Self, StdError> {
        let mut inner = conn::AddrIncoming::bind(&addr).map_err(|_| StdError::from(ErrorKind::NotFound))?;
        inner.set_nodelay(true);
        Ok(Self { inner })
    }
}

impl Stream for TcpIncoming {
    type Item = Result<conn::AddrStream, StdError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match ready!(Accept::poll_accept(Pin::new(&mut self.inner), cx)) {
            Some(Ok(s)) => Poll::Ready(Some(Ok(s))),
            Some(Err(e)) => Poll::Ready(Some(Err(e.into()))),
            None => Poll::Ready(None),
        }
    }
}

#[tokio::main(core_threads = 8)]
async fn main() {
    color_backtrace::install();
    env_logger::builder().format_timestamp_nanos().init();
    let swir_config = utils::config::Swir::new();

    let mc: MemoryChannel = utils::config::create_client_to_backend_channels(&swir_config);

    let client_ip = swir_config.client_ip.clone();
    let client_https_port: u16 = swir_config.client_https_port.clone();
    let client_http_port: u16 = swir_config.client_http_port.clone();
    let client_grpc_port: u16 = swir_config.client_grpc_port.clone();
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
    let tls = TlsAcceptor::from(Arc::new(config));

    let incoming = hyper::server::accept::from_stream::<_, _, StdError>(async_stream::try_stream! {
        let mut tcp = TcpIncoming::bind(client_https_addr)?;
        while let Some(stream) = tcp.try_next().await? {
            {
                let io = match boxio::connect(tls.clone(), stream.into_inner()).await {
                    Ok(io) => io,
                    Err(error) => {
                        error!("Unable to accept incoming connection. {:?}", error);
                        continue
                    },
                };
                yield BoxedIo::new(io);
                continue;
            }
            yield boxio::BoxedIo::new(stream)
        }
    });

    let to_client_sender_for_rest = mc.to_client_sender_for_rest.clone();
    let from_client_to_backend_channel_sender = mc.from_client_to_backend_channel_sender.clone();

    
    let http_service = make_service_fn(move |_| {
        let from_client_to_backend_channel_sender = from_client_to_backend_channel_sender.clone();
	let to_client_sender_for_rest = to_client_sender_for_rest.clone();
        async move {
            let from_client_to_backend_channel_sender = from_client_to_backend_channel_sender.clone();
	    let to_client_sender_for_rest = to_client_sender_for_rest;
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| handler(req, from_client_to_backend_channel_sender.clone(),to_client_sender_for_rest.clone())))
        }
    });

    let to_client_sender_for_rest = mc.to_client_sender_for_rest.clone();
    let from_client_to_backend_channel_sender = mc.from_client_to_backend_channel_sender.clone();
    
    let https_service = make_service_fn(move |_| {
        let from_client_to_backend_channel_sender = from_client_to_backend_channel_sender.clone();
	let to_client_sender_for_rest = to_client_sender_for_rest.clone();
        async move {
            let from_client_to_backend_channel_sender = from_client_to_backend_channel_sender.clone();
	    let to_client_sender_for_rest = to_client_sender_for_rest;
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| handler(req, from_client_to_backend_channel_sender.clone(),to_client_sender_for_rest.clone())))
        }
    });

    let from_client_to_backend_channel_sender = mc.from_client_to_backend_channel_sender.clone();
    let to_client_receiver_for_rest = mc.to_client_receiver_for_rest.clone();
    let to_client_receiver_for_grpc = mc.to_client_receiver_for_grpc.clone();
    let broker = async {
        backend_handlers::configure_broker(swir_config.channels,  mc).await;
    };

    let server = Server::bind(&client_http_addr).serve(http_service);
    let tls_server = Server::builder(incoming).serve(https_service);
    let client = async { client_handler(to_client_receiver_for_rest.clone()).await };
    let swir = grpc_handler::SwirAPI::new(from_client_to_backend_channel_sender, to_client_receiver_for_grpc);
    let svc = grpc_handler::client_api::client_api_server::ClientApiServer::new(swir);
    let grpc = tonic::transport::Server::builder().add_service(svc).serve(client_grpc_addr);

    if let Some(command) = client_executable {
        utils::command_utils::run_java_command(command);
    }

    let (_r1, _r2, _r3, _r4, _r5) = futures::join!(tls_server, server, client, broker, grpc);
}
