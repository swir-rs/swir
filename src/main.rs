//#![deny(warnings)]

#[macro_use]
extern crate log;

use std::{
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use std::io::{Error as StdError, ErrorKind};

use futures::lock::Mutex;
use futures_core::Stream;
use futures_util::{ready, TryStreamExt};
use hyper::{
    Body,
    Request, server::{accept::Accept, conn}, Server,
};
use hyper::service::{make_service_fn, service_fn};
use sled::Config;
use tokio::sync::mpsc;
use tokio_rustls::TlsAcceptor;

use boxio::BoxedIo;
use http_handler::client_handler;
use http_handler::handler;
use utils::pki_utils::{load_certs, load_private_key};

mod boxio;
mod grpc_handler;
mod http_handler;
mod messaging_handlers;
mod utils;

#[derive(Debug)]
struct TcpIncoming {
    inner: conn::AddrIncoming,
}

impl TcpIncoming {
    fn bind(addr: SocketAddr) -> Result<Self, StdError> {
        let mut inner =
            conn::AddrIncoming::bind(&addr).map_err(|_| StdError::from(ErrorKind::NotFound))?;
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

#[tokio::main]
async fn main() {
    env_logger::init();
    let swir_config = utils::config::Swir::new().unwrap();

    let client_ip = swir_config.client_ip;
    let client_https_port: u16 = swir_config.client_https_port;
    let client_http_port: u16 = swir_config.client_http_port;
    let client_grpc_port: u16 = swir_config.client_grpc_port;

    let consumer_topics = swir_config.messaging.kafka.consumer_topics;
    let brokers = swir_config.messaging.kafka.brokers;
    let client_executable = swir_config.client_executable;
    let producer_topics = swir_config.messaging.kafka.producer_topics;
    let consumer_group = swir_config.messaging.kafka.consumer_groups;

    let http_tls_certificate = swir_config.client_tls_certificate;
    let http_tls_key = swir_config.client_tls_private_key;

    let client_https_addr = std::net::SocketAddr::new(client_ip.parse().unwrap(), client_https_port);
    let client_http_addr =
        std::net::SocketAddr::new(client_ip.parse().unwrap(), client_http_port);
    let client_grpc_addr =
        std::net::SocketAddr::new(client_ip.parse().unwrap(), client_grpc_port);

    let config = Config::new().temporary(true);
    let db = config.open().unwrap();

    let certs = load_certs(http_tls_certificate).unwrap();
    // Load private key.
    let key = load_private_key(http_tls_key).unwrap();

    let mut config = rustls::ServerConfig::new(rustls::NoClientAuth::new());
    config
        .set_single_cert(certs, key)
        .expect("invalid key or certificate");
    let tls = TlsAcceptor::from(Arc::new(config));

    let incoming =
        hyper::server::accept::from_stream::<_, _, StdError>(async_stream::try_stream! {
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

    let (rest_to_msg_tx, rest_to_msg_rx): (
        mpsc::Sender<utils::structs::RestToMessagingContext>,
        mpsc::Receiver<utils::structs::RestToMessagingContext>,
    ) = mpsc::channel(1000);
    let (msg_to_rest_tx, msg_to_rest_rx): (
        mpsc::Sender<utils::structs::MessagingToRestContext>,
        mpsc::Receiver<utils::structs::MessagingToRestContext>,
    ) = mpsc::channel(1000);
    let (msg_to_grpc_tx, msg_to_grpc_rx): (
        mpsc::Sender<utils::structs::MessagingToRestContext>,
        mpsc::Receiver<utils::structs::MessagingToRestContext>,
    ) = mpsc::channel(1000);
    let msg_to_grpc_rx = Arc::new(Mutex::new(msg_to_grpc_rx));

    let tx = rest_to_msg_tx.clone();
    let http_service = make_service_fn(move |_| {
        let tx = tx.clone();
        async {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                handler(req, tx.clone())
            }))
        }
    });
    let tx = rest_to_msg_tx.clone();
    let https_service = make_service_fn(move |_| {
        let tx = tx.clone();
        async {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                handler(req, tx.clone())
            }))
        }
    });

    let broker = async {
        messaging_handlers::configure_broker(
            brokers,
            consumer_topics,
            producer_topics,
            consumer_group,
            db.clone(),
            rest_to_msg_rx,
            msg_to_rest_tx,
        )
            .await;
    };

    let server = Server::bind(&client_http_addr).serve(http_service);

    let tls_server = Server::builder(incoming).serve(https_service);

    let client = async { client_handler(msg_to_rest_rx).await };

    let swir = grpc_handler::SwirAPI {
        tx: rest_to_msg_tx,
        rx: msg_to_grpc_rx,
    };

    let svc = grpc_handler::client_api::clientapi_server::ClientApiServer::new(swir);
    let grpc = tonic::transport::Server::builder()
        .add_service(svc)
        .serve(client_grpc_addr);

    if let Some(command) = client_executable {
        utils::command_utils::run_java_command(command);
    }

    let (_r1, _r2, _r3, _r4, _r5) = futures::join!(tls_server, server, client, broker, grpc);
}
