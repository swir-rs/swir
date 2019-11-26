//#![deny(warnings)]
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

use std::sync;
use std::{
    fmt,
    future::Future,
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    // time::Duration,
};
use std::io::{Error as StdError, ErrorKind};
use std::thread;

use clap::App;
use crossbeam_channel::{Receiver, Sender, unbounded};
use futures_core::Stream;
use futures_executor::block_on;
#[macro_use]
use futures_util::{future, ready, TryFutureExt, TryStreamExt};
use futures_util::StreamExt;
use hyper::{Body, Error, Request, Server, server::{accept::Accept, conn}};
use hyper::service::{make_service_fn, service_fn};
use tokio_executor::Executor;
use tokio_net::tcp::TcpListener;
use tokio_rustls::TlsAcceptor;

use boxio::BoxedIo;
//use http_handler::client_handler;
use http_handler::handler;
use utils::pki_utils::{load_certs, load_private_key};

mod boxio;
mod http_handler;
mod messaging_handlers;
mod utils;

#[derive(Debug)]
struct TcpIncoming {
    inner: conn::AddrIncoming,
}

impl TcpIncoming {
    fn bind(addr: SocketAddr) -> Result<Self, StdError> {
        let mut inner = conn::AddrIncoming::bind(&addr).map_err(|e| StdError::from(ErrorKind::NotFound))?;
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
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();
    info!("Application arguments {:?}", matches);

    let external_address = matches.value_of("address").unwrap();
    let tls_port: u16 = matches.value_of("tlsport").unwrap_or_default().parse().expect("Unable to parse socket port");
    let plain_port: u16 = matches.value_of("plainport").unwrap_or_default().parse().expect("Unable to parse socket port");
    let sending_topic = matches.value_of("sending_topic").unwrap();
    let broker_address = matches.value_of("broker").unwrap();
    let command = matches.value_of("execute_command").unwrap();
    let receiving_topic = matches.value_of("receiving_topic").unwrap();


    let receiving_group = matches.value_of("receiving_group").unwrap_or_default();
    let http_tls_certificate = matches.value_of("http_tls_certificate").unwrap_or_default();
    let http_tls_key = matches.value_of("http_tls_key").unwrap();

    let tls_socket_addr = std::net::SocketAddr::new(external_address.parse().unwrap(), tls_port);
    let plain_socket_addr = std::net::SocketAddr::new(external_address.parse().unwrap(), plain_port);

    let addr = plain_socket_addr;

    info!("Using kafka broker on {}", broker_address);
    info!("Tls port Listening on {}", tls_socket_addr);
    info!("Plain port Listening on {}", plain_socket_addr);

    let certs = load_certs(&http_tls_certificate).unwrap();
    // Load private key.
    let key = load_private_key(&http_tls_key).unwrap();

    let mut config = rustls::ServerConfig::new(rustls::NoClientAuth::new());
    config.set_single_cert(certs, key).expect("invalid key or certificate");
    let tls = TlsAcceptor::from(Arc::new(config));

    let incoming = hyper::server::accept::from_stream(
        async_stream::try_stream! {
            let mut tcp = TcpIncoming::bind(tls_socket_addr)?;
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
        }
    );

    let (rest_to_msg_tx, rest_to_msg_rx): (Sender<utils::structs::RestToMessagingContext>, Receiver<utils::structs::RestToMessagingContext>) = unbounded();
    let (msg_to_rest_tx, msg_to_rest_rx): (Sender<utils::structs::MessagingToRestContext>, Receiver<utils::structs::MessagingToRestContext>) = unbounded();



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

    let server = Server::bind(&plain_socket_addr)
        .serve(http_service);

    let tls_server = Server::builder(incoming)
        .serve(https_service);


    futures::join!(tls_server, server);


//    let fut = async  {
//        server.await;
//    };
//    let http_plain_server_runtime = thread::spawn(||{
//        block_on(fut)
//        }
//    );


//    utils::command_utils::run_java_command(command.to_string());


    //https_server_runtime.join().unwrap_err();
//    http_plain_server_runtime.join().unwrap_err();
}

