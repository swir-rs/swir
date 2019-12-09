use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::stream::{Stream, StreamExt, TryStreamExt};
use http::HeaderValue;
use hyper::{Body, Client, HeaderMap, Method, Request, Response, StatusCode};
use hyper::client::connect::dns::GaiResolver;
use hyper::client::HttpConnector;

use crate::utils::structs::{Job, MessagingResult, PublishRequest, RestToMessagingContext};
use crate::utils::structs::MessagingToRestContext;

fn validate_content_type(headers: &HeaderMap<HeaderValue>) -> Option<bool> {
    match headers.get(http::header::CONTENT_TYPE) {
        Some(header) => {
            if header == HeaderValue::from_static("application/json") {
                return Some(true);
            } else {
                return None;
            }
        }
        None => return None
    }
}

async fn get_whole_body(mut req: Request<Body>) -> Vec<u8> {
    let mut whole_body = Vec::new();
    while let Some(maybe_chunk) = req.body_mut().next().await {
        if let Ok(chunk) = &maybe_chunk {
            whole_body.extend_from_slice(chunk);
        }
    }
    whole_body
}

pub async fn handler(req: Request<Body>, mut sender: mpsc::Sender<RestToMessagingContext>) -> Result<Response<Body>, hyper::Error> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_ACCEPTABLE;

    let headers = req.headers();
    debug!("Headers {:?}", headers);

    if validate_content_type(headers).is_none() {
        return Ok(response)
    }

    debug!("Body {:?}", req.body());
    let url = req.uri().clone().to_string();
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/publish") => {
            let whole_body = get_whole_body(req).await;

            let p = PublishRequest { payload: whole_body, url: url };
            debug!("{:?}", p);
            let (local_tx, mut local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
            let job = RestToMessagingContext { job: Job::Publish(p), sender: local_tx };

            if let Err(e) = sender.try_send(job) {
                warn!("Channel is dead {:?}", e);
                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                *response.body_mut() = Body::empty();
            }

            debug!("Waiting for broker");
            let response_from_broker: Result<MessagingResult, oneshot::Canceled> = local_rx.await;
            debug!("Got result {:?}", response_from_broker);
            if let Ok(res) = response_from_broker {
                *response.body_mut() = Body::from(res.result);
                *response.status_mut() = StatusCode::OK;
            } else {
                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                *response.body_mut() = Body::empty();
            }
            return Ok(response);
        }

        (&Method::POST, "/subscribe") => {
            let whole_body = get_whole_body(req).await;
            let maybe_json = serde_json::from_slice(&whole_body);
            match maybe_json {
                Ok(json) => {
                    info!("{:?}", json);
                    let (local_tx, mut local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
                    let job = RestToMessagingContext { job: Job::Subscribe(json), sender: local_tx };
                    info!("About to send to kafka processor");
                    if let Err(e) = sender.try_send(job) {
                        warn!("Channel is dead {:?}", e);
                        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        *response.body_mut() = Body::empty();
                    }
                    info!("Waiting for response from kafka");
                    let response_from_broker: Result<MessagingResult, oneshot::Canceled> = local_rx.await;
                    debug!("Got result {:?}", response_from_broker);
                    if let Ok(res) = response_from_broker {
                        *response.body_mut() = Body::from(res.result);
                        *response.status_mut() = StatusCode::OK;
                    } else {
                        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        *response.body_mut() = Body::empty();
                    }
                }
                Err(e) => {
                    warn!("{:?}", e);
                    *response.status_mut() = StatusCode::BAD_REQUEST;
                    *response.body_mut() = Body::from(e.to_string());
                }
            }
            Ok(response)
        }

        // The 404 Not Found route...
        _ => {
            let not_found = Response::default();
            *response.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}


async fn send_request(client: Client<HttpConnector<GaiResolver>>, payload: MessagingToRestContext) {
    let uri = payload.uri;
    let url = uri.parse::<hyper::Uri>().unwrap();

    let p = payload.payload.clone();
    let req = Request::builder()
        .method("POST")
        .uri(url)
        .header(hyper::header::CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .body(Body::from(payload.payload))
        .expect("request builder");

//    let sender = payload.sender.clone();
//    let err_sender = payload.sender.clone();


        let p = p.clone();
        info!("Making request for {}", String::from_utf8_lossy(&p));
        let resp = client.request(req).await;
        let res = resp.map(move |res| {
            debug!("Status POST to the client: {}", res.status());
//                let mut status = "All good".to_string();
            if res.status() != hyper::StatusCode::OK {
                warn!("Error from the client {}", res.status());
//                    status = "Invalid response from the client".to_string();
            }
//                if let Err(e) = sender.send(MessagingResult { status: u32::from(res.status().as_u16()), result: status }) {
//                    warn!("Problem with an internal communication {:?}", e);
//                }
//                res.into_body().concat2()
            res.into_body()
        }).map(|_| {})
            .map_err(move |err| {
                eprintln!("Error {}", err);
//                    if let Err(e) = err_sender.send(MessagingResult { status: 1, result: "Something is wrong".to_string() }) {
//                        warn!("Problem with an internal communication {:?}", e);
//                    }
            });
}

pub async fn client_handler(mut rx: mpsc::Receiver<MessagingToRestContext>) {
    let client = hyper::Client::builder().keep_alive(true).build_http();
    info!("Client done");
    while let Some(payload) = rx.next().await {
        send_request(client.clone(), payload).await
    }
}